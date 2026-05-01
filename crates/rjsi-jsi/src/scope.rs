use crate::{Context, Engine};

pub struct Scope<'cx, 'rt, E: Engine> {
    cx: &'cx mut Context<'rt, E>,
}

impl<'cx, 'rt, E: Engine> Scope<'cx, 'rt, E> {
    pub fn new(cx: &'cx mut Context<'rt, E>) -> Self {
        Self { cx }
    }

    pub fn cx(&mut self) -> &mut Context<'rt, E> {
        self.cx
    }
}

pub struct CallbackCx<'cx, 'rt, E: Engine> {
    scope: Scope<'cx, 'rt, E>,
}

impl<'cx, 'rt, E: Engine> CallbackCx<'cx, 'rt, E> {
    pub fn new(scope: Scope<'cx, 'rt, E>) -> Self {
        Self { scope }
    }

    pub fn scope(&mut self) -> &mut Scope<'cx, 'rt, E> {
        &mut self.scope
    }

    pub fn cx(&mut self) -> &mut Context<'rt, E> {
        self.scope.cx()
    }
}

pub trait ScopeKind {}

pub struct HandleScope;

pub struct CallbackScope;

pub struct TryCatchScope;

pub struct EscapableScope;

pub struct ModuleScope;

impl ScopeKind for HandleScope {}
impl ScopeKind for CallbackScope {}
impl ScopeKind for TryCatchScope {}
impl ScopeKind for EscapableScope {}
impl ScopeKind for ModuleScope {}

pub trait CanThrow<E: Engine> {
    fn throw(&mut self, err: E::Value<'_>);
}

pub trait CanEscape<E: Engine> {
    fn escape<T>(&mut self, value: T) -> T;
}

pub trait CanScheduleMicrotask<E: Engine> {
    fn schedule_microtask<'a>(&mut self, job: E::Function<'a>);
}

pub trait TryCatch<E: Engine> {
    fn has_exception(&self) -> bool;
    fn exception(&mut self) -> Option<E::Value<'_>>;
    fn clear_exception(&mut self);
}
