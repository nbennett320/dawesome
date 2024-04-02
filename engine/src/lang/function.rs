use crate::lang::chunk::Chunk;

#[derive(Debug, Clone)]
pub enum FunctionType {
    Fn,
    Script,
}

#[derive(Debug, Clone)]
pub struct Function {
    pub num_params: usize,
    pub chunk: Chunk,
    pub name: String,
    pub native: bool,
    pub function_type: FunctionType,
}

impl Function {
    pub fn new(name: String, function_type: FunctionType) -> Function {
        Function {
            num_params: 0,
            chunk: Chunk::new(),
            name,
            native: false,
            function_type,
        }
    }
}
