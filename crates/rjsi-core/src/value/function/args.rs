use crate::{IntoJsValue, JsContext, JsEngine, function::JsParameterType};
use smallvec::SmallVec;

pub type JsArgsVec<E> = SmallVec<[<E as JsEngine>::Value; 4]>;

pub trait IntoJsArg<'js, E: JsEngine> {
    fn push_js_arg(self, ctx: JsContext<'js, E>, vec: &mut JsArgsVec<E>);
}

impl<'js, E, T> IntoJsArg<'js, E> for T
where
    E: JsEngine,
    T: IntoJsValue<'js, E>,
    T: JsParameterType,
{
    fn push_js_arg(self, ctx: JsContext<'js, E>, vec: &mut JsArgsVec<E>) {
        vec.push(self.into_js_value(ctx).into_inner());
    }
}

impl<'js, E, T> IntoJsArg<'js, E> for Vec<T>
where
    E: JsEngine,
    T: IntoJsValue<'js, E>,
{
    fn push_js_arg(self, ctx: JsContext<'js, E>, vec: &mut JsArgsVec<E>) {
        vec.extend(
            self.into_iter()
                .map(|item| item.into_js_value(ctx.clone()).into_inner()),
        );
    }
}

pub trait IntoJsArgs<'js, E: JsEngine> {
    fn into_js_args(self, ctx: JsContext<'js, E>) -> JsArgsVec<E>;
}

macro_rules! impl_into_js_args {
    ($($T:ident),*) => {
        impl<'js, Eng: JsEngine, $($T),*> IntoJsArgs<'js, Eng> for ($($T,)*)
        where
            $($T: IntoJsArg<'js, Eng>),*
        {
            #[allow(unused_variables)]
            fn into_js_args(self, ctx: JsContext<'js, Eng>) -> JsArgsVec<Eng> {
                #[allow(non_snake_case)]
                let ($($T,)*) = self;
                #[allow(unused_mut)]
                let mut args = JsArgsVec::<Eng>::new();
                $($T.push_js_arg(ctx.clone(), &mut args);)*
                args
            }
        }
    };
}

impl_into_js_args!();
impl_into_js_args!(T1);
impl_into_js_args!(T1, T2);
impl_into_js_args!(T1, T2, T3);
impl_into_js_args!(T1, T2, T3, T4);
impl_into_js_args!(T1, T2, T3, T4, T5);
impl_into_js_args!(T1, T2, T3, T4, T5, T6);
impl_into_js_args!(T1, T2, T3, T4, T5, T6, T7);
impl_into_js_args!(T1, T2, T3, T4, T5, T6, T7, T8);
