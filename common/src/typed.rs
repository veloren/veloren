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
