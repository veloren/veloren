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
        $( $constr:ident $( ( $( $arg_name:ident : $arg_ty:ty ),* ) )? = $index:tt ),* $(,)?
    }) => {
        $crate::as_item! {
            $( #[$ty_attr] )*
            $vis enum $ty {
                $( $constr $( ($( $arg_ty, )*) )? = $index, )*
            }
        }

        #[allow(non_snake_case)]
        $vis mod $mod {
            use ::serde::{Deserialize, Serialize};

            pub trait PackedElim {
                $( type $constr; )*
            }

            #[derive(Serialize, Deserialize)]
            pub struct Cases<Elim: PackedElim> {
                $( pub $constr : Elim::$constr, )*
            }

            impl<T> PackedElim for $crate::util::Pure<T> {
                $( type $constr = T; )*
            }

            pub type PureCases<Elim> = Cases<$crate::util::Pure<Elim>>;
        }

        #[allow(unused_parens)]
        impl<'a, Elim: $mod::PackedElim, Context, Type, S>
            $crate::util::Typed<Context, Type, S> for $crate::util::ElimCase<&'a $ty, &'a $mod::Cases<Elim>, Type>
            where
                $( &'a Elim::$constr: $crate::util::Typed<($( ($( &'a $arg_ty, )*), )? Context), Type, S>, )*
        {
            fn reduce(self, context: Context) -> (Type, S)
            {
                let Self { expr, cases, .. } = self;
                match expr {
                    $( $ty::$constr $( ($( $arg_name, )*) )? =>
                        <_ as $crate::util::Typed<_, Type, _>>::reduce(
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
                $crate::util::ElimCase<&'a $ty, &'a $mod::Cases<Elim>, Type> : $crate::util::Typed<Context, Type, S>,
            {
                use $crate::util::Typed;

                let case = $crate::util::ElimCase {
                    expr: self,
                    cases,
                    ty: ::core::marker::PhantomData,
                };
                case.reduce(context)
            }

            pub fn elim_case_pure<'a, Type>(&'a self, cases: &'a $mod::PureCases<Type>) -> &'a Type
            {
                let ($crate::util::Pure(expr), ()) = self.elim_case(cases, ());
                expr
            }
        }
    }
}
