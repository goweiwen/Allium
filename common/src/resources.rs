use std::{
    cell::{Ref, RefCell},
    rc::Rc,
};

use type_map::TypeMap;

#[derive(Debug, Clone)]
pub struct Resources(pub Rc<RefCell<TypeMap>>);

impl Resources {
    /// Creates a new resource map.
    pub fn new(map: TypeMap) -> Self {
        Self(Rc::new(RefCell::new(map)))
    }

    /// Gets a resource from the resource map. Panics if the resource is not present.
    pub fn get<T: 'static>(&self) -> Ref<T> {
        Ref::map(self.0.borrow(), |x| x.get::<T>().unwrap())
    }

    /// Sets a resource in the resource map.
    pub fn set<T: 'static>(&self, value: T) {
        self.0.borrow_mut().insert(value);
    }
}
