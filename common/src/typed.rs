use core::marker::PhantomData;
use serde::{Deserialize, Serialize};

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

/// Given a head expression (Self) and a target type (Type),
/// attempt to synthesize a term that reduces head into the target type.
///
/// How we do this depends on the type of the head expression:
///
/// - For enums, we synthesize a match on the current head.  For each match arm,
///   we then repeat this process on the constructor arguments; if there are no
///   constructor arguments, we synthesize a literal (Pure term). (TODO: Handle
///   > 1 tuple properly--for now we just synthesize a Pure term for these
///   cases).
///
/// - For structs, we synthesize a projection on the current head.  For each
///   projection, we then repeat this process on the type of the projected
///   field.
///
/// - For other types (which currently have to opt out during the field
///   declaration), we synthesize a literal.
///
/// TODO: Differentiate between the context and the stack at some point; for
/// now, we only use the context as a stack.
pub trait SynthTyped<Context, Target> {
    type Expr;
}

/// Weak head reduction type (equivalent to applying a reduction to the head
/// variable, but this way we don't have to implement variable lookup and it
/// doesn't serialize with variables).
#[fundamental]
#[derive(Deserialize, Serialize)]
#[serde(transparent)]
pub struct WeakHead<Reduction, Type> {
    pub red: Reduction,
    #[serde(skip)]
    pub ty: PhantomData<Type>,
}

#[derive(Deserialize, Serialize)]
#[serde(transparent)]
pub struct Pure<T>(pub T);

impl<'a, Context: SubContext<S>, T, S> Typed<Context, &'a T, S> for &'a Pure<T> {
    fn reduce(self, context: Context) -> (&'a T, S) { (&self.0, context.sub_context()) }
}

impl<Context, Target> SynthTyped<Context, Target> for WeakHead<Pure<Target>, Target> {
    type Expr = Pure<Target>;
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
///         #[typed(pure)] Constr2(arg : u8) = 1,
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
///     use serde::{Deserialize, Serialize};
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
/// Unfortunately, due to restrictions on macro_rules, we currently always
/// require the types defined to #[repr(inttype)] as you can see above.  There
/// are also some other current limitations that we hopefully will be able to
/// lift at some point; struct variants are not yet supported, and neither
/// attributes on fields.
#[fundamental]
#[derive(Deserialize, Serialize)]
#[serde(transparent)]
pub struct ElimCase<Cases> {
    pub cases: Cases,
}

#[fundamental]
#[derive(Deserialize, Serialize)]
#[serde(transparent)]
pub struct ElimProj<Proj> {
    pub proj: Proj,
}
pub type ElimWeak<Type, Elim> = <WeakHead<Type, Elim> as SynthTyped<((Type,), ()), Elim>>::Expr;

#[macro_export]
macro_rules! as_item {
    ($i:item) => {
        $i
    };
}

#[macro_export]
/// This macro is used internally by typed.
///
/// We use this in order to reliably construct a "representative" type for the
/// weak head reduction type.  We need this because for some types of arguments
/// (empty variants for an enum, fields or constructor arguments explicitly
/// marked as #[typed(pure)], etc.) won't directly implement the WeakHead trait;
/// in such cases, we just synthesize a literal of the appropriate type.
macro_rules! make_weak_head_type {
    ($Target:ty, $( #[$attr:meta] )* , ) => {
        $crate::typed::Pure<$Target>
    };
    ($Target:ty, #[ typed(pure) ] , $( $extra:tt )*) => {
        $crate::typed::Pure<$Target>
    };
    ($Target:ty, , $Type:ty, $( $extra:tt )*) => {
        $crate::typed::Pure<$Target>
    };
    ($Target:ty, , $Type:ty) => {
        $Type
    }
}

#[macro_export]
macro_rules! make_case_elim {
    ($mod:ident, $( #[ $ty_attr:meta ] )* $vis:vis enum $ty:ident {
        $( $( #[$( $constr_attr:tt )*] )* $constr:ident $( ( $( $arg_name:ident : $arg_ty:ty ),* ) )? = $index:expr ),* $(,)?
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

            pub type PureCases<Elim> = $crate::typed::ElimCase<Cases<$crate::typed::Pure<Elim>>>;
        }

        impl<T> $mod::PackedElim for $crate::typed::Pure<T> {
            $( type $constr = $crate::typed::Pure<T>; )*
        }

        #[allow(unused_parens)]
        impl<Target> $mod::PackedElim for $crate::typed::WeakHead<$ty, Target>
            where $(
                $crate::typed::WeakHead<$crate::make_weak_head_type!(Target, $( #[$( $constr_attr )*] )* , $( $( $arg_ty ),* )?), Target> :
                $crate::typed::SynthTyped<($( ($( $arg_ty, )*), )? ()), Target>,
            )*
        {
            $( type $constr =
               <$crate::typed::WeakHead<$crate::make_weak_head_type!(Target, $( #[$( $constr_attr )*] )* , $( $( $arg_ty ),* )?), Target>
               as $crate::typed::SynthTyped<($( ($( $arg_ty, )*), )? ()), Target>>::Expr;
            )*
        }

        #[allow(unused_parens)]
        impl<Context, Target> $crate::typed::SynthTyped<(($ty,), Context), Target> for $crate::typed::WeakHead<$ty, Target>
            where $(
                $crate::typed::WeakHead<$crate::make_weak_head_type!(Target, $( #[$( $constr_attr )*] )* , $( $( $arg_ty ),* )?), Target> :
                $crate::typed::SynthTyped<($( ($( $arg_ty, )*), )? ()), Target>,
            )*
        {
            type Expr = $crate::typed::ElimCase<$mod::Cases<$crate::typed::WeakHead<$ty, Target>>>;
        }

        #[allow(unused_parens)]
        impl<'a, 'b, Elim: $mod::PackedElim, Context, Type, S>
            $crate::typed::Typed<((&'a $ty,), Context), Type, S> for &'b $crate::typed::ElimCase<$mod::Cases<Elim>>
            where
                $( &'b Elim::$constr: $crate::typed::Typed<($( ($( &'a $arg_ty, )*), )? Context), Type, S> ),*
        {
            fn reduce(self, ((head,), context): ((&'a $ty,), Context)) -> (Type, S)
            {
                match head {
                    $( $ty::$constr $( ($( $arg_name, )*) )? =>
                        <_ as $crate::typed::Typed<_, Type, _>>::reduce(
                            &self.cases.$constr,
                            ($( ($( $arg_name, )*), )? context),
                        ),
                    )*
                }
            }
        }

        impl $ty {
            pub fn elim<'a, Elim, Context, S, Type>(&'a self, elim: Elim, context: Context) -> (Type, S)
            where
                Elim : $crate::typed::Typed<((&'a $ty,), Context), Type, S>,
            {
                elim.reduce(((self,), context))
            }

            pub fn elim_case_pure<'a, 'b, Type>(&'a self, cases: &'b $mod::PureCases<Type>) -> &'b Type
            {
                let (expr, ()) = self.elim(cases, ());
                expr
            }

            #[allow(unused_parens)]
            pub fn elim_case_weak<'a, 'b, Type>(&'a self, cases: &'b $crate::typed::ElimWeak<Self, Type>) -> &'b Type
            where $(
                $crate::typed::WeakHead<$crate::make_weak_head_type!(Type, $( #[$( $constr_attr )*] )* , $( $( $arg_ty ),* )?), Type> :
                $crate::typed::SynthTyped<($( ($( $arg_ty, )*), )? ()), Type>,
            )*
                &'b $crate::typed::ElimWeak<Self, Type> : $crate::typed::Typed<((&'a $ty,), ()), &'b Type, ()>,
            {
                let (expr, ()) = self.elim(cases, ());
                expr
            }
        }
    }
}

#[macro_export]
macro_rules! make_proj_elim {
    ($mod:ident, $( #[ $ty_attr:meta ] )* $vis:vis struct $ty:ident {
        $( $( #[$( $constr_attr:tt )*] )* $field_vis:vis $constr:ident : $arg_ty:ty ),* $(,)?
    }) => {
        $crate::as_item! {
            $( #[$ty_attr] )*
            $vis struct $ty {
                $( $field_vis $constr : $arg_ty, )*
            }
        }

        #[allow(non_camel_case_types)]
        #[allow(dead_code)]
        $vis mod $mod {
            use ::serde::{Deserialize, Serialize};

            pub trait PackedElim {
                $( type $constr; )*
            }

            #[derive(Serialize, Deserialize)]
            pub enum Proj<Elim: PackedElim> {
                $( $constr(Elim::$constr), )*
            }

            pub type PureProj<Elim> = $crate::typed::ElimProj<Proj<$crate::typed::Pure<Elim>>>;
        }

        impl<T> $mod::PackedElim for $crate::typed::Pure<T> {
            $( type $constr = $crate::typed::Pure<T>; )*
        }

        #[allow(unused_parens)]
        impl<Target> $mod::PackedElim for $crate::typed::WeakHead<$ty, Target>
            where $(
                $crate::typed::WeakHead<$crate::make_weak_head_type!(Target, $( #[$( $constr_attr )*] )* , $arg_ty), Target> :
                $crate::typed::SynthTyped<(($arg_ty,), ()), Target>,
            )*
        {
            $( type $constr =
               <$crate::typed::WeakHead<$crate::make_weak_head_type!(Target, $( #[$( $constr_attr )*] )* , $arg_ty), Target>
               as $crate::typed::SynthTyped<(($arg_ty,), ()), Target>>::Expr;
            )*
        }

        #[allow(unused_parens)]
        impl<Context, Target> $crate::typed::SynthTyped<(($ty,), Context), Target> for $crate::typed::WeakHead<$ty, Target>
            where $(
                $crate::typed::WeakHead<$crate::make_weak_head_type!(Target, $( #[$( $constr_attr )*] )* , $arg_ty), Target> :
                $crate::typed::SynthTyped<(($arg_ty,), ()), Target>,
            )*
        {
            type Expr = $crate::typed::ElimProj<$mod::Proj<$crate::typed::WeakHead<$ty, Target>>>;
        }

        #[allow(unused_parens)]
        impl<'a, 'b, Elim: $mod::PackedElim, Context, Type, S>
            $crate::typed::Typed<((&'a $ty,), Context), Type, S> for &'b $crate::typed::ElimProj<$mod::Proj<Elim>>
            where
                $( &'b Elim::$constr: $crate::typed::Typed<((&'a $arg_ty,), Context), Type, S> ),*
        {
            fn reduce(self, ((head,), context): ((&'a $ty,), Context)) -> (Type, S)
            {
                match self.proj {
                    $( $mod::Proj::$constr(ref projection) =>
                        <_ as $crate::typed::Typed<_, Type, _>>::reduce(
                            projection,
                            ((&head.$constr,), context),
                        ),
                    )*
                }
            }
        }

        impl $ty {
            pub fn elim<'a, Elim, Context, S, Type>(&'a self, elim: Elim, context: Context) -> (Type, S)
            where
                Elim : $crate::typed::Typed<((&'a $ty,), Context), Type, S>,
            {
                elim.reduce(((self,), context))
            }

            pub fn elim_proj_pure<'a, 'b, Type>(&'a self, cases: &'b $mod::PureProj<Type>) -> &'b Type
            {
                let (expr, ()) = self.elim(cases, ());
                expr
            }

            #[allow(unused_parens)]
            pub fn elim_proj_weak<'a, 'b, Type>(&'a self, cases: &'b $crate::typed::ElimWeak<Self, Type>) -> &'b Type
            where $(
                $crate::typed::WeakHead<$crate::make_weak_head_type!(Type, $( #[$( $constr_attr )*] )* , $arg_ty), Type> :
                $crate::typed::SynthTyped<(($arg_ty,), ()), Type>,
            )*
                &'b $crate::typed::ElimWeak<Self, Type> : $crate::typed::Typed<((&'a $ty,), ()), &'b Type, ()>,
            {
                let (expr, ()) = self.elim(cases, ());
                expr
            }
        }
    }
}
