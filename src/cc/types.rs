use std::collections::HashMap;

use super::ast::*;

#[derive(Debug, Clone)]
pub struct StructLayout {
    pub fields: Vec<(String, CType)>,
    pub field_offsets: HashMap<String, u32>,
    pub total_slots: u32,
}

#[derive(Debug, Clone)]
pub struct FuncSig {
    pub return_ty: CType,
    pub params: Vec<CType>,
    pub is_variadic: bool,
}

pub struct TypeContext {
    pub structs: HashMap<String, StructLayout>,
    pub functions: HashMap<String, FuncSig>,
}

impl TypeContext {
    pub fn build(program: &Program) -> Result<Self, String> {
        let mut structs = HashMap::new();
        let mut functions = HashMap::new();

        for sd in &program.structs {
            let mut offsets = HashMap::new();
            let mut offset = 0u32;
            for (name, ty) in &sd.fields {
                offsets.insert(name.clone(), offset);
                offset += slot_count(ty, &structs);
            }
            structs.insert(
                sd.name.clone(),
                StructLayout {
                    fields: sd.fields.clone(),
                    field_offsets: offsets,
                    total_slots: offset,
                },
            );
        }

        for f in &program.functions {
            functions.insert(
                f.name.clone(),
                FuncSig {
                    return_ty: f.return_ty.clone(),
                    params: f.params.iter().map(|p| p.ty.clone()).collect(),
                    is_variadic: f.is_variadic,
                },
            );
        }

        Ok(Self { structs, functions })
    }
}

pub fn slot_count(ty: &CType, structs: &HashMap<String, StructLayout>) -> u32 {
    match ty {
        CType::Void => 0,
        CType::Int | CType::Char | CType::Pointer(_) => 1,
        CType::Array(elem, count) => slot_count(elem, structs) * count,
        CType::Struct(name) => {
            if let Some(layout) = structs.get(name) {
                layout.total_slots
            } else {
                1
            }
        }
    }
}
