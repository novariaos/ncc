use std::collections::HashMap;

use super::ast::*;
use super::types::*;
use crate::nvm::asm::AsmBuilder;

const INTRINSICS: &[(&str, &str)] = &[
    ("__nvm_exit", "exit"),
    ("__nvm_print", "print"),
    ("__nvm_spawn", "spawn"),
    ("__nvm_open", "open"),
    ("__nvm_read", "read"),
    ("__nvm_write", "write"),
    ("__nvm_create", "create"),
    ("__nvm_delete", "delete"),
    ("__nvm_cap_request", "cap_request"),
    ("__nvm_cap_spawn", "cap_spawn"),
    ("__nvm_msg_send", "msg_send"),
    ("__nvm_msg_receive", "msg_recieve"),
    ("__nvm_inb", "inb"),
    ("__nvm_outb", "outb"),
];

fn find_intrinsic(name: &str) -> Option<&'static str> {
    INTRINSICS.iter().find(|(n, _)| *n == name).map(|(_, sc)| *sc)
}

const TTY_FD_SLOT: u8 = 1;

struct GlobalSlots {
    map: HashMap<String, u16>,
    next_slot: u16,
}

impl GlobalSlots {
    fn new() -> Self {
        Self {
            map: HashMap::new(),
            next_slot: 2,
        }
    }

    fn assign(&mut self, name: &str, count: u32) -> u16 {
        if let Some(&slot) = self.map.get(name) {
            return slot;
        }
        let base = self.next_slot;
        self.next_slot += count as u16;
        self.map.insert(name.to_string(), base);
        base
    }

    fn get(&self, name: &str) -> Option<u16> {
        self.map.get(name).copied()
    }
}

struct FuncLocals {
    vars: HashMap<String, LocalVar>,
    next_slot: u8,
}

struct LocalVar {
    slot: u8,
    ty: CType,
    slot_count: u8,
}

impl FuncLocals {
    fn new() -> Self {
        Self {
            vars: HashMap::new(),
            next_slot: 0,
        }
    }

    fn alloc(&mut self, name: &str, ty: &CType, structs: &HashMap<String, StructLayout>) -> u8 {
        let count = slot_count(ty, structs) as u8;
        let slot = self.next_slot;
        self.next_slot += count;
        self.vars.insert(
            name.to_string(),
            LocalVar {
                slot,
                ty: ty.clone(),
                slot_count: count,
            },
        );
        slot
    }

    fn get(&self, name: &str) -> Option<&LocalVar> {
        self.vars.get(name)
    }
}

pub struct Codegen {
    asm: AsmBuilder,
    globals: GlobalSlots,
    types: TypeContext,
    label_counter: u32,
    called_functions: std::collections::HashSet<String>,
    break_label: Option<String>,
}

impl Codegen {
    pub fn generate(program: &Program, types: TypeContext) -> Result<String, String> {
        let mut cg = Codegen {
            asm: AsmBuilder::new(),
            globals: GlobalSlots::new(),
            types,
            label_counter: 0,
            called_functions: std::collections::HashSet::new(),
            break_label: None,
        };

        cg.collect_calls(program);

        for g in &program.globals {
            let count = slot_count(&g.ty, &cg.types.structs) as u32;
            cg.globals.assign(&g.name, count);
        }

        cg.asm.directive(".NVM0");
        cg.asm.blank();

        cg.emit_open_tty();

        for g in &program.globals {
            if let Some(init) = &g.init {
                let slot = cg.globals.get(&g.name).unwrap();
                match init {
                    Expr::IntLit(val) => {
                        cg.asm.emit_i32("push", *val);
                        cg.asm.emit_u8("store", slot as u8);
                    }
                    Expr::InitList(items) => {
                        for (i, item) in items.iter().enumerate() {
                            if let Expr::IntLit(val) = item {
                                cg.asm.emit_i32("push", *val);
                                cg.asm.emit_u8("store", (slot + i as u16) as u8);
                            }
                        }
                    }
                    _ => {}
                }
            }
        }

        cg.asm.emit_label("call", "main");
        cg.asm.emit_u8("load", 0);
        cg.asm.emit_syscall("exit");
        cg.asm.emit("hlt");
        cg.asm.blank();

        for func in &program.functions {
            if func.body.is_none() {
                continue;
            }
            if func.is_static && func.name != "main" && !cg.called_functions.contains(&func.name) {
                continue;
            }
            cg.emit_function(func)?;
        }

        Ok(cg.asm.finish())
    }

    fn emit_open_tty(&mut self) {
        self.asm.emit_i32("push", 0);
        self.asm.emit_i32("push", '/' as i32);
        self.asm.emit_i32("push", 'd' as i32);
        self.asm.emit_i32("push", 'e' as i32);
        self.asm.emit_i32("push", 'v' as i32);
        self.asm.emit_i32("push", '/' as i32);
        self.asm.emit_i32("push", 't' as i32);
        self.asm.emit_i32("push", 't' as i32);
        self.asm.emit_i32("push", 'y' as i32);
        self.asm.emit_syscall("open");
        self.asm.emit_u8("store", TTY_FD_SLOT);
        self.asm.blank();
    }

    fn collect_calls(&mut self, program: &Program) {
        for func in &program.functions {
            if let Some(body) = &func.body {
                self.collect_calls_in_block(body);
            }
        }
    }

    fn collect_calls_in_block(&mut self, block: &Block) {
        for stmt in block {
            self.collect_calls_in_stmt(stmt);
        }
    }

    fn collect_calls_in_stmt(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::Expr(e) | Stmt::Return(Some(e)) => self.collect_calls_in_expr(e),
            Stmt::Return(None) => {}
            Stmt::Local { init: Some(e), .. } => self.collect_calls_in_expr(e),
            Stmt::Local { init: None, .. } => {}
            Stmt::If { cond, then_body, else_body } => {
                self.collect_calls_in_expr(cond);
                self.collect_calls_in_block(then_body);
                if let Some(eb) = else_body {
                    self.collect_calls_in_block(eb);
                }
            }
            Stmt::While { cond, body } => {
                self.collect_calls_in_expr(cond);
                self.collect_calls_in_block(body);
            }
            Stmt::DoWhile { body, cond } => {
                self.collect_calls_in_block(body);
                self.collect_calls_in_expr(cond);
            }
            Stmt::For { init, cond, step, body } => {
                if let Some(i) = init {
                    self.collect_calls_in_stmt(i);
                }
                if let Some(c) = cond {
                    self.collect_calls_in_expr(c);
                }
                if let Some(s) = step {
                    self.collect_calls_in_expr(s);
                }
                self.collect_calls_in_block(body);
            }
            Stmt::Block(b) => self.collect_calls_in_block(b),
            Stmt::Switch { expr, cases, default } => {
                self.collect_calls_in_expr(expr);
                for (_, body) in cases {
                    self.collect_calls_in_block(body);
                }
                if let Some(def) = default {
                    self.collect_calls_in_block(def);
                }
            }
            Stmt::Break => {}
        }
    }

    fn collect_calls_in_expr(&mut self, expr: &Expr) {
        match expr {
            Expr::Call { func, args } => {
                self.called_functions.insert(func.clone());
                for a in args {
                    self.collect_calls_in_expr(a);
                }
            }
            Expr::BinOp { lhs, rhs, .. } => {
                self.collect_calls_in_expr(lhs);
                self.collect_calls_in_expr(rhs);
            }
            Expr::UnaryOp { expr, .. }
            | Expr::Deref(expr)
            | Expr::AddrOf(expr)
            | Expr::PreIncDec { expr, .. }
            | Expr::PostIncDec { expr, .. }
            | Expr::Cast { expr, .. }
            | Expr::SizeofExpr(expr) => {
                self.collect_calls_in_expr(expr);
            }
            Expr::Assign { target, value } | Expr::CompoundAssign { target, value, .. } => {
                self.collect_calls_in_expr(target);
                self.collect_calls_in_expr(value);
            }
            Expr::Index { array, index } => {
                self.collect_calls_in_expr(array);
                self.collect_calls_in_expr(index);
            }
            Expr::Field { expr, .. } | Expr::ArrowField { expr, .. } => {
                self.collect_calls_in_expr(expr);
            }
            _ => {}
        }
    }

    fn fresh_label(&mut self, prefix: &str) -> String {
        let id = self.label_counter;
        self.label_counter += 1;
        format!(".L{}_{}", prefix, id)
    }

    fn emit_function(&mut self, func: &FuncDef) -> Result<(), String> {
        let body = func.body.as_ref().unwrap();

        let mut locals = FuncLocals::new();

        for param in &func.params {
            locals.alloc(&param.name, &param.ty, &self.types.structs);
        }

        self.prescan_locals(body, &mut locals);

        let frame_size = locals.next_slot;

        self.asm.label(&func.name);
        self.asm.emit_u8("enter", frame_size);

        for (i, param) in func.params.iter().enumerate() {
            if let Some(lv) = locals.get(&param.name) {
                self.asm.emit_u8("load_arg", i as u8);
                self.asm.emit_u8("store_rel", lv.slot);
            }
        }

        self.emit_block(body, &locals)?;

        self.asm.emit_i32("push", 0);
        self.asm.emit_u8("store", 0);
        self.asm.emit("leave");
        self.asm.emit("ret");
        self.asm.blank();

        Ok(())
    }

    fn prescan_locals(&self, block: &Block, locals: &mut FuncLocals) {
        for stmt in block {
            match stmt {
                Stmt::Local { name, ty, .. } => {
                    locals.alloc(name, ty, &self.types.structs);
                }
                Stmt::If { then_body, else_body, .. } => {
                    self.prescan_locals(then_body, locals);
                    if let Some(eb) = else_body {
                        self.prescan_locals(eb, locals);
                    }
                }
                Stmt::While { body, .. } | Stmt::DoWhile { body, .. } => {
                    self.prescan_locals(body, locals);
                }
                Stmt::For { init, body, .. } => {
                    if let Some(init) = init {
                        if let Stmt::Local { name, ty, .. } = init.as_ref() {
                            locals.alloc(name, ty, &self.types.structs);
                        }
                    }
                    self.prescan_locals(body, locals);
                }
                Stmt::Block(b) => self.prescan_locals(b, locals),
                Stmt::Switch { cases, default, .. } => {
                    for (_, body) in cases {
                        self.prescan_locals(body, locals);
                    }
                    if let Some(def) = default {
                        self.prescan_locals(def, locals);
                    }
                }
                _ => {}
            }
        }
    }

    fn emit_block(&mut self, block: &Block, locals: &FuncLocals) -> Result<(), String> {
        for stmt in block {
            self.emit_stmt(stmt, locals)?;
        }
        Ok(())
    }

    fn emit_stmt(&mut self, stmt: &Stmt, locals: &FuncLocals) -> Result<(), String> {
        match stmt {
            Stmt::Local { name, init, .. } => {
                if let Some(expr) = init {
                    let lv = locals.get(name).ok_or(format!("Unknown local: {}", name))?;
                    match expr {
                        Expr::InitList(items) => {
                            for (i, item) in items.iter().enumerate() {
                                self.emit_expr(item, locals)?;
                                self.asm.emit_u8("store_rel", lv.slot + i as u8);
                            }
                        }
                        _ => {
                            self.emit_expr(expr, locals)?;
                            self.asm.emit_u8("store_rel", lv.slot);
                        }
                    }
                }
            }
            Stmt::Expr(expr) => {
                self.emit_expr(expr, locals)?;
                if self.expr_produces_value(expr) {
                    self.asm.emit("pop");
                }
            }
            Stmt::Return(val) => {
                if let Some(expr) = val {
                    self.emit_expr(expr, locals)?;
                } else {
                    self.asm.emit_i32("push", 0);
                }
                self.asm.emit_u8("store", 0);
                self.asm.emit("leave");
                self.asm.emit("ret");
            }
            Stmt::If { cond, then_body, else_body } => {
                let else_label = self.fresh_label("else");
                let end_label = self.fresh_label("endif");

                self.emit_expr(cond, locals)?;
                if else_body.is_some() {
                    self.asm.emit_label("jz", &else_label);
                } else {
                    self.asm.emit_label("jz", &end_label);
                }

                self.emit_block(then_body, locals)?;

                if let Some(eb) = else_body {
                    self.asm.emit_label("jmp", &end_label);
                    self.asm.label(&else_label);
                    self.emit_block(eb, locals)?;
                }

                self.asm.label(&end_label);
            }
            Stmt::While { cond, body } => {
                let loop_label = self.fresh_label("while");
                let end_label = self.fresh_label("wend");
                let old_break = self.break_label.take();
                self.break_label = Some(end_label.clone());

                self.asm.label(&loop_label);
                self.emit_expr(cond, locals)?;
                self.asm.emit_label("jz", &end_label);
                self.emit_block(body, locals)?;
                self.asm.emit_label("jmp", &loop_label);
                self.asm.label(&end_label);
                self.break_label = old_break;
            }
            Stmt::DoWhile { body, cond } => {
                let loop_label = self.fresh_label("do");
                let end_label = self.fresh_label("doend");
                let old_break = self.break_label.take();
                self.break_label = Some(end_label.clone());

                self.asm.label(&loop_label);
                self.emit_block(body, locals)?;
                self.emit_expr(cond, locals)?;
                self.asm.emit_label("jnz", &loop_label);
                self.asm.label(&end_label);
                self.break_label = old_break;
            }
            Stmt::For { init, cond, step, body } => {
                let loop_label = self.fresh_label("for");
                let end_label = self.fresh_label("forend");
                let old_break = self.break_label.take();
                self.break_label = Some(end_label.clone());

                if let Some(init) = init {
                    self.emit_stmt(init, locals)?;
                }

                self.asm.label(&loop_label);

                if let Some(cond) = cond {
                    self.emit_expr(cond, locals)?;
                    self.asm.emit_label("jz", &end_label);
                }

                self.emit_block(body, locals)?;

                if let Some(step) = step {
                    self.emit_expr(step, locals)?;
                    if self.expr_produces_value(step) {
                        self.asm.emit("pop");
                    }
                }

                self.asm.emit_label("jmp", &loop_label);
                self.asm.label(&end_label);
                self.break_label = old_break;
            }
            Stmt::Block(b) => {
                self.emit_block(b, locals)?;
            }
            Stmt::Switch { expr, cases, default } => {
                let end_label = self.fresh_label("sw_end");
                let old_break = self.break_label.take();
                self.break_label = Some(end_label.clone());

                self.emit_expr(expr, locals)?;

                let mut case_labels: Vec<String> = Vec::new();
                let default_label = self.fresh_label("sw_def");

                for (val, _) in cases.iter() {
                    let lbl = self.fresh_label("sw_c");
                    self.asm.emit("dup");
                    self.asm.emit_i32("push", *val);
                    self.asm.emit("eq");
                    self.asm.emit_label("jnz", &lbl);
                    case_labels.push(lbl);
                }

                if default.is_some() {
                    self.asm.emit_label("jmp", &default_label);
                } else {
                    self.asm.emit("pop");
                    self.asm.emit_label("jmp", &end_label);
                }

                for (i, (_, body)) in cases.iter().enumerate() {
                    self.asm.label(&case_labels[i]);
                    self.asm.emit("pop");
                    self.emit_block(body, locals)?;
                }

                if let Some(def_body) = default {
                    self.asm.label(&default_label);
                    self.asm.emit("pop");
                    self.emit_block(def_body, locals)?;
                }

                self.asm.label(&end_label);
                self.break_label = old_break;
            }
            Stmt::Break => {
                if let Some(label) = &self.break_label {
                    self.asm.emit_label("jmp", &label.clone());
                }
            }
        }
        Ok(())
    }

    fn emit_expr(&mut self, expr: &Expr, locals: &FuncLocals) -> Result<(), String> {
        match expr {
            Expr::IntLit(val) => {
                self.asm.emit_i32("push", *val);
            }
            Expr::CharLit(val) => {
                self.asm.emit_i32("push", *val);
            }
            Expr::StrLit(_) => {
                self.asm.emit_i32("push", 0);
            }
            Expr::Var(name) => {
                if let Some(lv) = locals.get(name) {
                    self.asm.emit_u8("load_rel", lv.slot);
                } else if let Some(slot) = self.globals.get(name) {
                    self.asm.emit_u8("load", slot as u8);
                } else {
                    return Err(format!("Undefined variable: {}", name));
                }
            }
            Expr::BinOp { op, lhs, rhs } => {
                match op {
                    BinOp::LogicalAnd => {
                        let false_label = self.fresh_label("and_f");
                        let end_label = self.fresh_label("and_e");
                        self.emit_expr(lhs, locals)?;
                        self.asm.emit_label("jz", &false_label);
                        self.emit_expr(rhs, locals)?;
                        self.asm.emit_label("jmp", &end_label);
                        self.asm.label(&false_label);
                        self.asm.emit_i32("push", 0);
                        self.asm.label(&end_label);
                        return Ok(());
                    }
                    BinOp::LogicalOr => {
                        let true_label = self.fresh_label("or_t");
                        let end_label = self.fresh_label("or_e");
                        self.emit_expr(lhs, locals)?;
                        self.asm.emit_label("jnz", &true_label);
                        self.emit_expr(rhs, locals)?;
                        self.asm.emit_label("jmp", &end_label);
                        self.asm.label(&true_label);
                        self.asm.emit_i32("push", 1);
                        self.asm.label(&end_label);
                        return Ok(());
                    }
                    _ => {}
                }

                self.emit_expr(lhs, locals)?;
                self.emit_expr(rhs, locals)?;
                match op {
                    BinOp::Add => self.asm.emit("add"),
                    BinOp::Sub => self.asm.emit("sub"),
                    BinOp::Mul => self.asm.emit("mul"),
                    BinOp::Div => self.asm.emit("div"),
                    BinOp::Mod => self.asm.emit("mod"),
                    BinOp::Eq => self.asm.emit("eq"),
                    BinOp::Neq => self.asm.emit("neq"),
                    BinOp::Lt => self.asm.emit("lt"),
                    BinOp::Gt => self.asm.emit("gt"),
                    BinOp::LtEq => {
                        self.asm.emit("gt");
                        self.asm.emit_i32("push", 0);
                        self.asm.emit("eq");
                    }
                    BinOp::GtEq => {
                        self.asm.emit("lt");
                        self.asm.emit_i32("push", 0);
                        self.asm.emit("eq");
                    }
                    BinOp::LogicalAnd | BinOp::LogicalOr => unreachable!(),
                }
            }
            Expr::UnaryOp { op, expr: inner } => {
                match op {
                    UnaryOp::Neg => {
                        self.asm.emit_i32("push", 0);
                        self.emit_expr(inner, locals)?;
                        self.asm.emit("sub");
                    }
                    UnaryOp::Not => {
                        self.emit_expr(inner, locals)?;
                        self.asm.emit_i32("push", 0);
                        self.asm.emit("eq");
                    }
                }
            }
            Expr::Call { func, args } => {
                self.emit_call(func, args, locals)?;
            }
            Expr::Index { array, index } => {
                self.emit_array_read(array, index, locals)?;
            }
            Expr::Field { expr: inner, name } => {
                self.emit_struct_field_read(inner, name, locals)?;
            }
            Expr::Assign { target, value } => {
                self.emit_assign(target, value, locals)?;
            }
            Expr::CompoundAssign { op, target, value } => {
                self.emit_compound_assign(*op, target, value, locals)?;
            }
            Expr::PostIncDec { op, expr: inner } => {
                self.emit_post_incdec(*op, inner, locals)?;
            }
            Expr::PreIncDec { op, expr: inner } => {
                self.emit_pre_incdec(*op, inner, locals)?;
            }
            Expr::Cast { expr: inner, .. } => {
                self.emit_expr(inner, locals)?;
            }
            Expr::SizeofType(ty) => {
                let sz = slot_count(ty, &self.types.structs);
                self.asm.emit_i32("push", (sz * 4) as i32);
            }
            Expr::SizeofExpr(_) => {
                self.asm.emit_i32("push", 4);
            }
            Expr::AddrOf(_) | Expr::Deref(_) | Expr::ArrowField { .. } => {
                return Err(format!("Pointer operations not fully supported yet: {:?}", expr));
            }
            Expr::InitList(_) => {
                self.asm.emit_i32("push", 0);
            }
        }
        Ok(())
    }

    fn expr_produces_value(&self, expr: &Expr) -> bool {
        match expr {
            Expr::Call { func, .. } => func != "printf",
            Expr::Assign { .. } | Expr::CompoundAssign { .. } => true,
            Expr::PostIncDec { .. } | Expr::PreIncDec { .. } => true,
            _ => true,
        }
    }

    fn emit_assign(&mut self, target: &Expr, value: &Expr, locals: &FuncLocals) -> Result<(), String> {
        self.emit_expr(value, locals)?;
        self.asm.emit("dup");
        self.emit_store_to(target, locals)?;
        Ok(())
    }

    fn emit_compound_assign(&mut self, op: BinOp, target: &Expr, value: &Expr, locals: &FuncLocals) -> Result<(), String> {
        self.emit_expr(target, locals)?;
        self.emit_expr(value, locals)?;
        match op {
            BinOp::Add => self.asm.emit("add"),
            BinOp::Sub => self.asm.emit("sub"),
            BinOp::Mul => self.asm.emit("mul"),
            BinOp::Div => self.asm.emit("div"),
            BinOp::Mod => self.asm.emit("mod"),
            _ => return Err(format!("Invalid compound assign op: {:?}", op)),
        }
        self.asm.emit("dup");
        self.emit_store_to(target, locals)?;
        Ok(())
    }

    fn emit_post_incdec(&mut self, op: IncDec, target: &Expr, locals: &FuncLocals) -> Result<(), String> {
        self.emit_expr(target, locals)?;
        self.emit_expr(target, locals)?;
        self.asm.emit_i32("push", 1);
        match op {
            IncDec::Inc => self.asm.emit("add"),
            IncDec::Dec => self.asm.emit("sub"),
        }
        self.emit_store_to(target, locals)?;
        Ok(())
    }

    fn emit_pre_incdec(&mut self, op: IncDec, target: &Expr, locals: &FuncLocals) -> Result<(), String> {
        self.emit_expr(target, locals)?;
        self.asm.emit_i32("push", 1);
        match op {
            IncDec::Inc => self.asm.emit("add"),
            IncDec::Dec => self.asm.emit("sub"),
        }
        self.asm.emit("dup");
        self.emit_store_to(target, locals)?;
        Ok(())
    }

    fn emit_store_to(&mut self, target: &Expr, locals: &FuncLocals) -> Result<(), String> {
        match target {
            Expr::Var(name) => {
                if let Some(lv) = locals.get(name) {
                    self.asm.emit_u8("store_rel", lv.slot);
                } else if let Some(slot) = self.globals.get(name) {
                    self.asm.emit_u8("store", slot as u8);
                } else {
                    return Err(format!("Undefined variable: {}", name));
                }
            }
            Expr::Index { array, index } => {
                self.emit_array_write_from_stack(array, index, locals)?;
            }
            Expr::Field { expr, name } => {
                self.emit_struct_field_write_from_stack(expr, name, locals)?;
            }
            _ => return Err(format!("Invalid assignment target: {:?}", target)),
        }
        Ok(())
    }

    fn emit_call(&mut self, func: &str, args: &[Expr], locals: &FuncLocals) -> Result<(), String> {
        if func == "printf" {
            return self.emit_printf(args, locals);
        }

        if func == "__nvm_tty_fd" {
            self.asm.emit_u8("load", TTY_FD_SLOT);
            return Ok(());
        }

        if let Some(syscall_name) = find_intrinsic(func) {
            for arg in args {
                self.emit_expr(arg, locals)?;
            }
            self.asm.emit_syscall(syscall_name);
            return Ok(());
        }

        for arg in args {
            self.emit_expr(arg, locals)?;
        }
        self.asm.emit_label("call", func);
        for _ in 0..args.len() {
            self.asm.emit("pop");
        }
        self.asm.emit_u8("load", 0);

        Ok(())
    }

    fn emit_tty_fd(&mut self) {
        self.asm.emit_u8("load", TTY_FD_SLOT);
    }

    fn emit_printf(&mut self, args: &[Expr], locals: &FuncLocals) -> Result<(), String> {
        if args.is_empty() {
            return Err("printf requires at least one argument".to_string());
        }

        let fmt = match &args[0] {
            Expr::StrLit(s) => s.clone(),
            _ => return Err("printf format string must be a string literal".to_string()),
        };

        let mut arg_idx = 1;
        let mut chars = fmt.chars().peekable();

        while let Some(ch) = chars.next() {
            if ch == '%' {
                match chars.next() {
                    Some('d') | Some('i') => {
                        if arg_idx < args.len() {
                            self.emit_expr(&args[arg_idx], locals)?;
                            self.asm.emit_label("call", "print_int");
                            self.asm.emit("pop");
                            arg_idx += 1;
                        }
                    }
                    Some('c') => {
                        if arg_idx < args.len() {
                            self.emit_tty_fd();
                            self.emit_expr(&args[arg_idx], locals)?;
                            self.asm.emit_syscall("write");
                            self.asm.emit("pop");
                            arg_idx += 1;
                        }
                    }
                    Some('s') => {
                        if arg_idx < args.len() {
                            if let Expr::StrLit(s) = &args[arg_idx] {
                                for byte in s.bytes() {
                                    self.emit_tty_fd();
                                    self.asm.emit_i32("push", byte as i32);
                                    self.asm.emit_syscall("write");
                                    self.asm.emit("pop");
                                }
                            }
                            arg_idx += 1;
                        }
                    }
                    Some('%') => {
                        self.emit_tty_fd();
                        self.asm.emit_i32("push", '%' as i32);
                        self.asm.emit_syscall("write");
                        self.asm.emit("pop");
                    }
                    Some(other) => {
                        return Err(format!("Unsupported printf format specifier: %{}", other));
                    }
                    None => {
                        return Err("Incomplete format specifier in printf".to_string());
                    }
                }
            } else {
                self.emit_tty_fd();
                self.asm.emit_i32("push", ch as i32);
                self.asm.emit_syscall("write");
                self.asm.emit("pop");
            }
        }

        Ok(())
    }

    fn emit_array_read(&mut self, array: &Expr, index: &Expr, locals: &FuncLocals) -> Result<(), String> {
        let (base_slot, size, is_global) = self.resolve_array_info(array, locals)?;

        match index {
            Expr::IntLit(i) => {
                let slot = base_slot + *i as u8;
                if is_global {
                    self.asm.emit_u8("load", slot);
                } else {
                    self.asm.emit_u8("load_rel", slot);
                }
            }
            _ => {
                self.emit_expr(index, locals)?;
                self.emit_array_read_dispatch(base_slot, size, is_global);
            }
        }
        Ok(())
    }

    fn emit_array_write_from_stack(&mut self, array: &Expr, index: &Expr, locals: &FuncLocals) -> Result<(), String> {
        let (base_slot, size, is_global) = self.resolve_array_info(array, locals)?;

        match index {
            Expr::IntLit(i) => {
                let slot = base_slot + *i as u8;
                if is_global {
                    self.asm.emit_u8("store", slot);
                } else {
                    self.asm.emit_u8("store_rel", slot);
                }
            }
            _ => {
                self.emit_expr(index, locals)?;
                self.emit_array_write_dispatch(base_slot, size, is_global);
            }
        }
        Ok(())
    }

    fn resolve_array_info(&self, array: &Expr, locals: &FuncLocals) -> Result<(u8, u32, bool), String> {
        match array {
            Expr::Var(name) => {
                if let Some(lv) = locals.get(name) {
                    let size = lv.slot_count as u32;
                    Ok((lv.slot, size, false))
                } else if let Some(slot) = self.globals.get(name) {
                    Ok((slot as u8, 1, true))
                } else {
                    Err(format!("Undefined array: {}", name))
                }
            }
            _ => Err(format!("Complex array base not supported: {:?}", array)),
        }
    }

    fn emit_array_read_dispatch(&mut self, base_slot: u8, size: u32, is_global: bool) {
        let done_label = self.fresh_label("ard");
        let mut case_labels = Vec::new();

        for _ in 0..size {
            case_labels.push(self.fresh_label("arc"));
        }
        let default_label = self.fresh_label("ard_def");

        for i in 0..size {
            self.asm.emit("dup");
            self.asm.emit_i32("push", i as i32);
            self.asm.emit("eq");
            self.asm.emit_label("jnz", &case_labels[i as usize]);
        }
        self.asm.emit_label("jmp", &default_label);

        for i in 0..size {
            self.asm.label(&case_labels[i as usize]);
            self.asm.emit("pop");
            let slot = base_slot + i as u8;
            if is_global {
                self.asm.emit_u8("load", slot);
            } else {
                self.asm.emit_u8("load_rel", slot);
            }
            self.asm.emit_label("jmp", &done_label);
        }

        self.asm.label(&default_label);
        self.asm.emit("pop");
        self.asm.emit_i32("push", 0);

        self.asm.label(&done_label);
    }

    fn emit_array_write_dispatch(&mut self, base_slot: u8, size: u32, is_global: bool) {
        let done_label = self.fresh_label("awd");
        let mut case_labels = Vec::new();

        for _ in 0..size {
            case_labels.push(self.fresh_label("awc"));
        }
        let default_label = self.fresh_label("awd_def");

        for i in 0..size {
            self.asm.emit("dup");
            self.asm.emit_i32("push", i as i32);
            self.asm.emit("eq");
            self.asm.emit_label("jnz", &case_labels[i as usize]);
        }
        self.asm.emit_label("jmp", &default_label);

        for i in 0..size {
            self.asm.label(&case_labels[i as usize]);
            self.asm.emit("pop");
            let slot = base_slot + i as u8;
            if is_global {
                self.asm.emit_u8("store", slot);
            } else {
                self.asm.emit_u8("store_rel", slot);
            }
            self.asm.emit_label("jmp", &done_label);
        }

        self.asm.label(&default_label);
        self.asm.emit("pop");
        self.asm.emit("pop");

        self.asm.label(&done_label);
    }

    fn emit_struct_field_read(&mut self, expr: &Expr, field_name: &str, locals: &FuncLocals) -> Result<(), String> {
        match expr {
            Expr::Var(var_name) => {
                let lv = locals.get(var_name)
                    .ok_or(format!("Undefined variable: {}", var_name))?;
                let struct_name = match &lv.ty {
                    CType::Struct(name) => name.clone(),
                    _ => return Err(format!("{} is not a struct", var_name)),
                };
                let layout = self.types.structs.get(&struct_name)
                    .ok_or(format!("Unknown struct: {}", struct_name))?;
                let offset = layout.field_offsets.get(field_name)
                    .ok_or(format!("Unknown field: {}.{}", struct_name, field_name))?;
                self.asm.emit_u8("load_rel", lv.slot + *offset as u8);
            }
            _ => return Err(format!("Complex struct expr not supported: {:?}", expr)),
        }
        Ok(())
    }

    fn emit_struct_field_write_from_stack(&mut self, expr: &Expr, field_name: &str, locals: &FuncLocals) -> Result<(), String> {
        match expr {
            Expr::Var(var_name) => {
                let lv = locals.get(var_name)
                    .ok_or(format!("Undefined variable: {}", var_name))?;
                let struct_name = match &lv.ty {
                    CType::Struct(name) => name.clone(),
                    _ => return Err(format!("{} is not a struct", var_name)),
                };
                let layout = self.types.structs.get(&struct_name)
                    .ok_or(format!("Unknown struct: {}", struct_name))?;
                let offset = layout.field_offsets.get(field_name)
                    .ok_or(format!("Unknown field: {}.{}", struct_name, field_name))?;
                self.asm.emit_u8("store_rel", lv.slot + *offset as u8);
            }
            _ => return Err(format!("Complex struct expr not supported: {:?}", expr)),
        }
        Ok(())
    }
}
