use std::{collections::HashMap};


#[derive(Debug, PartialEq)]
pub struct FishBasket(HashMap<String, u32>);


impl FishBasket {
    pub fn new() -> Self {
        Self(HashMap::new())
    }

    pub fn map(&self) -> &HashMap<String, u32> {
        &self.0
    }

    pub fn map_mut(&mut self) -> &mut HashMap<String, u32> {
        &mut self.0
    }
}

