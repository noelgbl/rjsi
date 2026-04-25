use crate::function::{Constructor, FromParams, IntoJsCallable, ParamsAccessor, RustFunc};
use crate::{
    HostError, JsArrayOps, JsContext, JsContextImpl, JsEngine, JsErrorFactory, JsExceptionThrower,
    JsFunc, JsObject, JsObjectOps, JsResult, JsTypeOf, JsValue, JsValueImpl, PropertyDescriptor,
    PropertyKey, RjsiJSError,
};

use std::any::TypeId;
use std::cell::{Ref, RefCell, RefMut};
use std::ops::Deref;

/// JsClass trait for rust type that supports TypeId
/// `TypeId` is currently only available for types which ascribe to `'static`,
pub trait JsClass<E: JsEngine>: Sized + 'static {
    // the name of class constructor
    const NAME: &'static str;

    /// Whether instances of this class can be called as functions.
    /// Override to `true` for callable classes (e.g. RustFunc).
    const CALLABLE: bool = false;

    /// Returns the data constructor function for this class
    fn data_constructor() -> Constructor<E>;

    /// Returns the implicit constructor function for this class
    ///
    /// This is the function called when the class is invoked directly without
    /// 'new' (e.g. `MyClass()`).
    ///
    /// Note: This is distinct from `data_constructor()`, which provides the internal
    /// constructor logic.
    fn call_without_new() -> Constructor<E> {
        Constructor::new(|| ())
    }

    /// Configures the class prototype and constructor with methods and properties
    fn class_setup(class: &ClassSetup<'_, '_, E>) -> JsResult<()>;

    /// Marks JavaScript values held by this object to prevent garbage collection.
    ///
    /// Used primarily by the QuickJS engine. Some engines like JavaScriptCore
    /// handle reference tracking automatically.
    ///
    /// IMPORTANT: Do NOT use clone() inside this method as it may break reference
    /// counting in the garbage collector.
    ///
    /// Default implementation does nothing. Override this when you need to prevent
    /// JS objects referenced by Rust from being garbage collected.
    fn gc_mark_with<F>(&self, _mark_fn: F)
    where
        F: FnMut(&JsValue<'_, E>),
    {
        // Default implementation does nothing
    }
}

#[doc(hidden)]
pub trait JsClassExt<E: JsEngine>: JsClass<E> {
    /// Shared constructor body that builds or returns the instance value.
    ///
    /// Engines with different native constructor ABIs can reuse this and keep
    /// their post-construction wiring local. For example, QuickJS/JSC call
    /// `constructor()`, while ArkJS calls `construct_value()` directly and
    /// finishes prototype synchronization in its own callback.
    fn construct_value<'js>(
        ctx: &'js E::Context,
        this: E::Value,
        args: Vec<E::Value>,
    ) -> Result<E::Value, E::Value>
    where
        E::Context: JsErrorFactory + JsExceptionThrower,
        E::Value: JsObjectOps + JsArrayOps + JsTypeOf,
    {
        let js_ctx = JsContext::<E>::new(E::raw_context_from_ref(ctx));
        let mut accessor = ParamsAccessor::new(js_ctx.clone(), this.clone(), args);

        if this.is_undefined() {
            match Self::call_without_new().0.call(&mut accessor) {
                Ok(v) => return Ok(v),
                Err(e) => return Err(e.throw_js_exception(js_ctx)),
            }
        }

        let instance = match Self::data_constructor().0.call(&mut accessor) {
            Ok(v) => {
                if v.is_exception() {
                    return Err(v);
                }
                v
            }
            Err(e) => return Err(e.throw_js_exception(js_ctx.clone())),
        };

        match JsObject::from_js_value(js_ctx.clone(), JsValue::from_raw(js_ctx.clone(), instance)) {
            Ok(obj) => Ok(obj.into_value()),
            Err(e) => Err(e.throw_js_exception(js_ctx)),
        }
    }

    /// Default constructor adapter for engines whose constructor callback
    /// receives the JS constructor object as `this`.
    fn constructor<'js>(ctx: &'js E::Context, this: E::Value, args: Vec<E::Value>) -> E::Value
    where
        E::Context: JsErrorFactory + JsExceptionThrower,
        E::Value: JsObjectOps + JsArrayOps + JsTypeOf,
    {
        let value = match Self::construct_value(ctx, this.clone(), args) {
            Ok(value) => value,
            Err(value) => return value,
        };

        if this.is_undefined() {
            return value;
        }

        let js_ctx = JsContext::<E>::new(E::raw_context_from_ref(ctx));
        let instance =
            match JsObject::from_js_value(js_ctx.clone(), JsValue::from_raw(js_ctx.clone(), value))
            {
                Ok(obj) => obj,
                Err(e) => return e.throw_js_exception(js_ctx.clone()),
            };

        let proto =
            match JsObject::from_js_value(js_ctx.clone(), JsValue::from_raw(js_ctx.clone(), this))
                .and_then(|constructor| constructor.get("prototype"))
            {
                Ok(proto) => proto,
                Err(e) => return e.throw_js_exception(js_ctx),
            };

        instance.prototype(proto);
        instance.into_value()
    }

    /// Free resources of a class instance by finalizer
    fn free(value: E::Value)
    where
        E::Value: JsObjectOps,
    {
        let value = value.clone();
        let ptr = value.get_opaque() as *mut RefCell<Self>;
        if !ptr.is_null() {
            unsafe {
                let _ = Box::from_raw(ptr);
            };
        }
    }

    /// call object as function
    fn call<'js>(
        ctx: &'js E::Context,
        function: E::Value,
        this: E::Value,
        args: Vec<E::Value>,
    ) -> E::Value
    where
        E::Value: JsObjectOps + JsArrayOps + 'static,
        E::Context: JsErrorFactory + JsExceptionThrower,
    {
        let js_ctx = JsContext::<E>::new(E::raw_context_from_ref(ctx));
        let mut accessor = ParamsAccessor::new(js_ctx.clone(), this, args);

        let obj = match JsObject::from_js_value(
            js_ctx.clone(),
            JsValue::from_raw(js_ctx.clone(), function),
        ) {
            Ok(obj) => obj,
            Err(e) => return e.throw_js_exception(js_ctx.clone()),
        };

        let mut func = match obj.borrow_mut::<RustFunc<E>>() {
            Ok(f) => f,
            Err(_) => {
                return RjsiJSError::from(HostError::not_function()).throw_js_exception(js_ctx);
            }
        };

        match func.call(&mut accessor) {
            Ok(v) => v,
            Err(e) => e.throw_js_exception(js_ctx),
        }
    }
}

// Blanket implementation
impl<T, E: JsEngine> JsClassExt<E> for T where T: JsClass<E> {}

/// Represents a JavaScript class constructor
///
/// This struct encapsulates a JavaScript object that serves as a class constructor.
/// It is used to create class instances and manage class lifecycle.
pub struct Class<'js, E: JsEngine>(pub(crate) JsObject<'js, E>);

impl<'js, E: JsEngine> Deref for Class<'js, E>
where
    E::Value: JsValueImpl,
{
    type Target = JsObject<'js, E>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'js, E: JsEngine> Class<'js, E>
where
    E::Value: JsValueImpl + JsObjectOps,
{
    /// Create a new instance of the class
    pub fn instance<JC: JsClass<E>>(self, value: JC) -> JsObject<'js, E> {
        let context = self.0.context();
        let ptr = Box::into_raw(Box::new(RefCell::new(value)));

        let instance = E::Value::make_instance(
            context.native_context(),
            self.0.clone().into_value(),
            ptr as _,
        );

        let _ = self
            .0
            .get::<_, JsObject<'js, E>>("prototype")
            .map(|proto| instance.set_prototype(proto.into_value()));
        JsObject::from_raw(context, instance)
    }

    /// Check if the object is an instance of the specified class
    pub fn instance_of<JC: JsClass<E>>(object: &JsObject<'js, E>) -> bool {
        let context = object.context();
        if let Ok(class) = Self::lookup::<JC>(&context) {
            object.as_value().instance_of(class.0.into_value())
        } else {
            false
        }
    }

    /// Returns the registered constructor for a class type.
    pub fn lookup<JC: JsClass<E>>(context: &JsContext<'js, E>) -> JsResult<Self> {
        let constructor = JsContextImpl::class_get(context.native_context(), TypeId::of::<JC>())
            .ok_or_else(|| {
                HostError::new(
                    crate::error::E_INVALID_STATE,
                    format!("JS Class {} is not registered", std::any::type_name::<JC>()),
                )
            })?;

        Ok(Self(JsObject::from_raw(context.clone(), constructor)))
    }

    pub fn prototype<JC: JsClass<E>>(context: &JsContext<'js, E>) -> JsResult<JsObject<'js, E>> {
        let class = Class::lookup::<JC>(context)?;
        class.0.get::<_, JsObject<'js, E>>("prototype")
    }

    /// Construct a Class constructor from a JsObject if it is an instance of the specified class
    pub fn from_object<JC: JsClass<E>>(obj: &JsObject<'js, E>) -> Option<Self> {
        if Self::instance_of::<JC>(obj) {
            Some(Self(obj.clone()))
        } else {
            None
        }
    }
}

impl<'js, E: JsEngine> JsObject<'js, E>
where
    E::Value: JsValueImpl + JsObjectOps,
{
    /// Borrow the underlying data from an instance
    pub fn borrow<T>(&self) -> JsResult<Ref<'_, T>>
    where
        T: JsClass<E>,
    {
        if !Class::instance_of::<T>(self) {
            return Err(HostError::new(
                crate::error::E_TYPE,
                format!("Not instance of {}", std::any::type_name::<T>()),
            )
            .with_name("TypeError")
            .into());
        }

        let ptr = self.as_value().get_opaque() as *mut RefCell<T>;
        if ptr.is_null() {
            Err(HostError::new(
                crate::error::E_INTERNAL,
                format!("Failed to borrow for type {}", std::any::type_name::<T>()),
            )
            .into())
        } else {
            // SAFETY: ptr was created by Box::into_raw in instance()
            unsafe { &*ptr }.try_borrow().map_err(|_| {
                HostError::new(
                    crate::error::E_INTERNAL,
                    format!("Failed to borrow for type {}", std::any::type_name::<T>()),
                )
                .into()
            })
        }
    }

    /// Mutably borrow the underlying data from an instance
    pub fn borrow_mut<T>(&self) -> JsResult<RefMut<'_, T>>
    where
        T: JsClass<E>,
    {
        if !Class::instance_of::<T>(self) {
            return Err(HostError::new(
                crate::error::E_TYPE,
                format!("Not instance of {}", std::any::type_name::<T>()),
            )
            .with_name("TypeError")
            .into());
        }

        let ptr = self.as_value().get_opaque() as *mut RefCell<T>;
        if ptr.is_null() {
            Err(HostError::new(
                crate::error::E_INTERNAL,
                format!("Failed to borrow for type {}", std::any::type_name::<T>()),
            )
            .into())
        } else {
            // SAFETY: ptr was created by Box::into_raw in instance()
            unsafe { &*ptr }.try_borrow_mut().map_err(|_| {
                HostError::new(
                    crate::error::E_INTERNAL,
                    format!("Failed to borrow for type {}", std::any::type_name::<T>()),
                )
                .into()
            })
        }
    }

    pub fn prototype(&self, proto: JsObject<'js, E>) -> bool {
        self.as_value().set_prototype(proto.into_value())
    }
}

pub struct ClassSetup<'a, 'js, E: JsEngine> {
    constructor: JsObject<'js, E>,
    prototype: JsObject<'js, E>,
    context: &'a JsContext<'js, E>,
}

impl<'a, 'js, E: JsEngine> ClassSetup<'a, 'js, E>
where
    E::Value: JsObjectOps,
{
    pub fn new(constructor: JsObject<'js, E>, context: &'a JsContext<'js, E>) -> JsResult<Self> {
        let constructor = Class(constructor);
        let prototype = constructor.0.get::<_, JsObject<'js, E>>("prototype")?;
        Ok(Self {
            constructor: constructor.0,
            prototype,
            context,
        })
    }

    /// Access the underlying JS context
    pub fn context(&self) -> &JsContext<'js, E> {
        self.context
    }

    /// Access the prototype object of this class
    pub fn prototype_object(&self) -> JsObject<'js, E> {
        self.prototype.clone()
    }

    pub fn method<F, P, K: 'static>(&self, name: &str, f: F) -> JsResult<()>
    where
        F: IntoJsCallable<E, P, K> + 'static,
        P: FromParams<E>,
        E: 'static,
    {
        let func = JsFunc::new(self.context.clone(), f)?;
        let func = func.name(name)?;
        self.prototype.set(name, func)?;
        Ok(())
    }

    pub fn callback_method<F>(&self, name: &str, arity: u32, callback: F) -> JsResult<()>
    where
        F: for<'i> FnMut(
                JsContext<'i, E>,
                Option<JsObject<'i, E>>,
                Vec<JsValue<'i, E>>,
            ) -> JsResult<JsValue<'i, E>>
            + 'static,
        E::Value: JsTypeOf + 'static,
        E: 'static,
    {
        let func = JsFunc::callback(self.context.clone(), arity, callback)?;
        let func = func.name(name)?;
        self.prototype.set(name, func)?;
        Ok(())
    }

    pub fn accessor_callback_method<F>(&self, name: &str, arity: u32, callback: F) -> JsResult<()>
    where
        F: for<'i> FnMut(&mut ParamsAccessor<'i, E>) -> JsResult<<E as JsEngine>::Value> + 'static,
        E::Value: JsTypeOf + 'static,
        E: 'static,
    {
        let func = JsFunc::accessor_callback(self.context.clone(), arity, callback)?;
        let func = func.name(name)?;
        self.prototype.set(name, func)?;
        Ok(())
    }

    pub fn static_method<F, P, K: 'static>(&self, name: &str, f: F) -> JsResult<()>
    where
        F: IntoJsCallable<E, P, K> + 'static,
        P: FromParams<E>,
        E: 'static,
    {
        let func = JsFunc::new(self.context.clone(), f)?;
        let func = func.name(name)?;
        self.constructor.set(name, func)?;
        Ok(())
    }

    pub fn callback_static_method<F>(&self, name: &str, arity: u32, callback: F) -> JsResult<()>
    where
        F: for<'i> FnMut(
                JsContext<'i, E>,
                Option<JsObject<'i, E>>,
                Vec<JsValue<'i, E>>,
            ) -> JsResult<JsValue<'i, E>>
            + 'static,
        E::Value: JsTypeOf + 'static,
        E: 'static,
    {
        let func = JsFunc::callback(self.context.clone(), arity, callback)?;
        let func = func.name(name)?;
        self.constructor.set(name, func)?;
        Ok(())
    }

    pub fn accessor_callback_static_method<F>(
        &self,
        name: &str,
        arity: u32,
        callback: F,
    ) -> JsResult<()>
    where
        F: for<'i> FnMut(&mut ParamsAccessor<'i, E>) -> JsResult<<E as JsEngine>::Value> + 'static,
        E::Value: JsTypeOf + 'static,
        E: 'static,
    {
        let func = JsFunc::accessor_callback(self.context.clone(), arity, callback)?;
        let func = func.name(name)?;
        self.constructor.set(name, func)?;
        Ok(())
    }

    pub fn property<Key>(&self, k: Key, descriptor: PropertyDescriptor<'js, E>) -> JsResult<()>
    where
        Key: for<'b> Into<PropertyKey<'b, E>>,
    {
        self.prototype.define_property(k, descriptor)?;
        Ok(())
    }

    pub fn static_property<Key>(
        &self,
        k: Key,
        descriptor: PropertyDescriptor<'js, E>,
    ) -> JsResult<()>
    where
        Key: for<'b> Into<PropertyKey<'b, E>>,
    {
        self.constructor.define_property(k, descriptor)?;
        Ok(())
    }

    pub fn new_func<F, P, K: 'static>(&self, f: F) -> JsResult<JsFunc<'js, E>>
    where
        F: IntoJsCallable<E, P, K> + 'static,
        P: FromParams<E>,
        E: 'static,
    {
        JsFunc::new(self.context.clone(), f)
    }

    pub fn new_callback_func<F>(&self, arity: u32, callback: F) -> JsResult<JsFunc<'js, E>>
    where
        F: for<'i> FnMut(
                JsContext<'i, E>,
                Option<JsObject<'i, E>>,
                Vec<JsValue<'i, E>>,
            ) -> JsResult<JsValue<'i, E>>
            + 'static,
        E::Value: JsTypeOf + 'static,
        E: 'static,
    {
        JsFunc::callback(self.context.clone(), arity, callback)
    }

    pub fn new_accessor_callback_func<F>(&self, arity: u32, callback: F) -> JsResult<JsFunc<'js, E>>
    where
        F: for<'i> FnMut(&mut ParamsAccessor<'i, E>) -> JsResult<<E as JsEngine>::Value> + 'static,
        E::Value: JsTypeOf + 'static,
        E: 'static,
    {
        JsFunc::accessor_callback(self.context.clone(), arity, callback)
    }
}
