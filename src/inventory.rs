use std::collections::HashMap;

/// Uma cesta de peixes, serve para armazenar todos os peixes do usuário
#[derive(Debug, PartialEq)]
pub struct FishBasket(HashMap<String, u32>);

impl FishBasket {
    /// Cria uma nova cesta
    pub fn new() -> Self {
        Self(HashMap::new())
    }

    /// Permite acessar as funções internas do HashMap
    pub fn map(&self) -> &HashMap<String, u32> {
        &self.0
    }

    /// Permite alterar o estado do hashmap interno
    pub fn map_mut(&mut self) -> &mut HashMap<String, u32> {
        &mut self.0
    }
}

impl Default for FishBasket {
    fn default() -> Self {
        Self::new()
    }
}
