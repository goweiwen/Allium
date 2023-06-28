use std::cell::{Ref, RefCell};
use std::rc::Rc;

use log::trace;
use type_map::TypeMap;

#[derive(Debug, Clone)]
pub struct Resources(pub Rc<RefCell<TypeMap>>);

impl Resources {
    /// Creates a new resource map.
    pub fn new(map: TypeMap) -> Self {
        Self(Rc::new(RefCell::new(map)))
    }

    /// Gets a ref to a resource from the resource map. Panics if the resource is not present.
    pub fn get<T: 'static>(&self) -> Ref<T> {
        trace!("getting ref to resource: {:?}", std::any::type_name::<T>());
        Ref::map(self.0.borrow(), |x| x.get::<T>().unwrap())
    }

    /// Sets a resource in the resource map.
    pub fn insert<T: 'static>(&self, value: T) {
        self.0.borrow_mut().insert(value);
    }
}
