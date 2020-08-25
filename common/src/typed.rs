use core::marker::PhantomData;

pub trait SubContext<Context> {
    fn sub_context(self) -> Context;
}

impl<Context> SubContext<Context> for Context {
    fn sub_context(self) -> Context { self }
}

impl<Head, Tail> SubContext<Tail> for (Head, Tail) {
    fn sub_context(self) -> Tail { self.1 }
}

pub trait Typed<Context, Type, S> {
    fn reduce(self, context: Context) -> (Type, S);
}

pub struct Pure<T>(pub T);

impl<Context: SubContext<S>, T, S> Typed<Context, Pure<T>, S> for T {
    fn reduce(self, context: Context) -> (Pure<T>, S) { (Pure(self), context.sub_context()) }
}

/// A lazy pattern match reified as a Rust type.
///
/// `expr` is the expression being matched on, generally of some enum type `Ty`.
///
/// `case` represents the pattern match--it will generally be a structure with
/// one field per constructor in `Ty`.  The field should contain enough
/// information to run the match arm for that constructor, given the information
/// contained in the constructor arguments.
///
/// `ty` represents the return type of the match expression.  It does not carry
/// any runtime-relevant information, but is needed in order to simplify our
/// trait definitions.
///
/// The intent is that you should not construct this structure directly, nor
/// should you define or construct the `Cases` structure directly.  Instead, to
/// use this you are expected to wrap your enum declaration in a call to
/// [make_case_elim!], as follows:
///
/// ```
///  # #![feature(arbitrary_enum_discriminant)]
///  # #[macro_use] extern crate veloren_common;
///
/// veloren_common::make_case_elim!(
///     my_type_module,
///     #[repr(u32)]
///     #[derive(Clone,Copy)]
///     pub enum MyType {
///         Constr1 = 0,
///         Constr2(arg : u8) = 1,
///         /* ..., */
///     }
/// );
/// ```
///
/// This macro automatically does a few things.  First, it creates the `enum`
/// type `MyType` in the current scope, as expected.  Second, it creates a
/// module named `my_type_module` in the current scope, into which it dumps a
/// few things.  In this case:
///
/// ```
/// # #![feature(arbitrary_enum_discriminant)]
/// # #[macro_use] extern crate veloren_common;
///
/// #[repr(u32)]
/// #[derive(Clone, Copy)]
/// pub enum MyType {
///     Constr1 = 0,
///     Constr2(u8) = 1,
///     /* ..., */
/// }
///
/// # #[allow(non_snake_case)]
/// # #[allow(dead_code)]
/// mod my_type_module {
///     use ::serde::{Deserialize, Serialize};
///
///     /// The number of variants in this enum.
///     pub const NUM_VARIANTS: usize = 2;
///
///     /// An array of all the variant indices (in theory, this can be used by this or other
///     /// macros in order to easily build up things like uniform random samplers).
///     pub const ALL_INDICES: [u32; NUM_VARIANTS] = [0, 1];
///
///     /// A convenience trait used to store a different type for each constructor in this
///     /// pattern.
///     pub trait PackedElim {
///         type Constr1;
///         type Constr2;
///     }
///
///     /// The actual *cases.*  If you think of pattern match arms as being closures that accept
///     /// the constructor types as arguments, you can think of this structure as somehow
///     /// representing just the data *owned* by the closure.  This is also what you will
///     /// generally store in your ron file--it has a field for each constructor of your enum,
///     /// with the types of all the fields specified by the implementation of [PackedElim] for
///     /// the [Elim] argument.  Each field has the same name as the constructor it represents.
///     #[derive(Serialize, Deserialize)]
///     pub struct Cases<Elim: PackedElim> {
///         pub Constr1: Elim::Constr1,
///         pub Constr2: Elim::Constr2,
///     }
///
///     /// Finally, because it represents by an overwhelming margin the most common usecase, we
///     /// predefine a particular pattern matching strategy--"pure"--where every arm holds data of
///     /// the exact same type, T.
///     impl<T> PackedElim for veloren_common::typed::Pure<T> {
///         type Constr1 = T;
///         type Constr2 = T;
///     }
///
///     /// Because PureCases is so convenient, we have an alias for it.  Thus, in order to
///     /// represent a pattern match on an argument that returns a constant of type (u8,u8,u8) for
///     /// each arm, you'd use the type `PureCases<(u8, u8, u8)>`.
///     pub type PureCases<Elim> = Cases<veloren_common::typed::Pure<Elim>>;
/// }
/// ```
///
/// Finally, a useful implementation of the [Typed] trait completes this story,
/// providing a way to evaluate this lazy math statement within Rust.
/// Unfortunately, [Typed] is quite complicated, and this story is still being
/// fully evaluated, so showing teh type may not be that elucidating.
/// Instead, we'll just present the method you can use most easily to pattern
/// match using the PureCases pattern we mentioned earlier:
///
/// pub fn elim_case_pure<'a, Type>(&'a self, cases: &'a $mod::PureCases<Type>)
/// -> &'a Type
///
/// If self is expression of your defined enum type, and match data defined by
/// PureCases, this evaluates the pattern match on self and returns the matched
/// case.
///
/// To see how this is used in more detail, check out
/// `common/src/body/humanoid.rs`; it is also used extensively in the world
/// repository.
///
/// ---
///
/// Limitations:
///
/// Unfortunately, due to restrictions on procedural macros, we currently always
/// require the types defined to #[repr(inttype)] as you can see above.  There
/// are also some other current limitations that we hopefully will be able to
/// lift at some point; struct variants are not yet supported, and neither
/// attributes on fields.
#[fundamental]
pub struct ElimCase<Expr, Cases, Type> {
    pub expr: Expr,
    pub cases: Cases,
    pub ty: PhantomData<Type>,
}

#[macro_export]
macro_rules! as_item {
    ($i:item) => {
        $i
    };
}

#[macro_export]
macro_rules! make_case_elim {
    ($mod:ident, $( #[$ty_attr:meta] )* $vis:vis enum $ty:ident {
        $( $constr:ident $( ( $( $arg_name:ident : $arg_ty:ty ),* ) )? = $index:expr ),* $(,)?
    }) => {
        $crate::as_item! {
            $( #[$ty_attr] )*
            $vis enum $ty {
                $( $constr $( ($( $arg_ty, )*) )? = $index, )*
            }
        }

        #[allow(non_snake_case)]
        #[allow(dead_code)]
        $vis mod $mod {
            use ::serde::{Deserialize, Serialize};

            pub const NUM_VARIANTS: usize = 0 $( + { let _ = $index; 1 } )*;

            pub const ALL_INDICES: [u32; NUM_VARIANTS] = [ $( $index, )* ];

            pub trait PackedElim {
                $( type $constr; )*
            }

            #[derive(Serialize, Deserialize)]
            pub struct Cases<Elim: PackedElim> {
                $( pub $constr : Elim::$constr, )*
            }

            impl<T> PackedElim for $crate::typed::Pure<T> {
                $( type $constr = T; )*
            }

            pub type PureCases<Elim> = Cases<$crate::typed::Pure<Elim>>;
        }

        #[allow(unused_parens)]
        impl<'a, Elim: $mod::PackedElim, Context, Type, S>
            $crate::typed::Typed<Context, Type, S> for $crate::typed::ElimCase<&'a $ty, &'a $mod::Cases<Elim>, Type>
            where
                $( &'a Elim::$constr: $crate::typed::Typed<($( ($( &'a $arg_ty, )*), )? Context), Type, S>, )*
        {
            fn reduce(self, context: Context) -> (Type, S)
            {
                let Self { expr, cases, .. } = self;
                match expr {
                    $( $ty::$constr $( ($( $arg_name, )*) )? =>
                        <_ as $crate::typed::Typed<_, Type, _>>::reduce(
                            &cases.$constr,
                            ($( ($( $arg_name, )*), )? context),
                        ),
                    )*
                }
            }
        }

        impl $ty {
            pub fn elim_case<'a, Elim: $mod::PackedElim, Context, S, Type>(&'a self, cases: &'a $mod::Cases<Elim>, context: Context) ->
                (Type, S)
            where
                $crate::typed::ElimCase<&'a $ty, &'a $mod::Cases<Elim>, Type> : $crate::typed::Typed<Context, Type, S>,
            {
                use $crate::typed::Typed;

                let case = $crate::typed::ElimCase {
                    expr: self,
                    cases,
                    ty: ::core::marker::PhantomData,
                };
                case.reduce(context)
            }

            pub fn elim_case_pure<'a, Type>(&'a self, cases: &'a $mod::PureCases<Type>) -> &'a Type
            {
                let ($crate::typed::Pure(expr), ()) = self.elim_case(cases, ());
                expr
            }
        }
    }
}
