use crate::vm::value::Value;
use crate::vm::opcode::MethodKind;

#[derive(Debug, Clone)]
pub enum JoinPred {
    Keys(String, String),
    Lambda(usize),
    Closure(usize, Vec<Value>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct VMColumn {
    pub name: String,
    pub ty: crate::frontend::ast::Type,
    pub is_auto: bool,
    pub is_pk: bool,
    pub is_unique: bool,
}

// Relational table object representation.
#[derive(Debug)]
pub struct TableObj {
    pub table_name: String,
    pub columns: Vec<VMColumn>,
    pub rows: Vec<Value>,
    pub sql_binding: Option<SqlBinding>,
    pub sql_where: Option<String>,
    pub pending_op: Option<MethodKind>,
}

impl Clone for TableObj {
    fn clone(&self) -> Self {
        for val in &self.rows {
            unsafe { val.inc_ref(); }
        }
        TableObj {
            table_name: self.table_name.clone(),
            columns: self.columns.clone(),
            rows: self.rows.clone(),
            sql_binding: self.sql_binding.clone(),
            sql_where: self.sql_where.clone(),
            pending_op: self.pending_op.clone(),
        }
    }
}

#[derive(Clone)]
pub struct SqlBinding {
    pub db_conn: std::sync::Arc<parking_lot::Mutex<rusqlite::Connection>>,
    pub table_name: String,
}

impl std::fmt::Debug for SqlBinding {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SqlBinding")
            .field("table_name", &self.table_name)
            .finish_non_exhaustive()
    }
}

impl TableObj {
    pub fn len(&self) -> usize {
        let cols = self.columns.len();
        if cols == 0 { 0 } else { self.rows.len() / cols }
    }

    pub fn to_json(&self) -> crate::vm::object::JsonVal {
        let col_keys: Vec<std::sync::Arc<String>> = self.columns
            .iter()
            .map(|c| crate::vm::object::intern_key(c.name.clone()))
            .collect();
        let num_cols = self.columns.len();
        let num_rows = self.len();
        let mut json_rows = Vec::with_capacity(num_rows);
        for r_idx in 0..num_rows {
            let mut obj = Vec::with_capacity(num_cols);
            for c_idx in 0..num_cols {
                let cell_idx = r_idx * num_cols + c_idx;
                obj.push((std::sync::Arc::clone(&col_keys[c_idx]), crate::vm::utils::json::value_to_json(&self.rows[cell_idx])));
            }
            json_rows.push(crate::vm::object::JsonVal::Object(std::sync::Arc::new(parking_lot::RwLock::new(obj))));
        }
        crate::vm::object::JsonVal::Array(std::sync::Arc::new(parking_lot::RwLock::new(json_rows)))
    }

    pub fn to_formatted_grid(&self) -> String {
        if self.columns.is_empty() { return String::new(); }
        let mut widths: Vec<usize> = self.columns.iter().map(|c| c.name.len()).collect();
        let num_cols = self.columns.len();
        let num_rows = self.len();
        for r_idx in 0..num_rows {
            for c_idx in 0..num_cols {
                let cell_idx = r_idx * num_cols + c_idx;
                widths[c_idx] = widths[c_idx].max(self.rows[cell_idx].to_string().len());
            }
        }
        
        let mut s = String::new();
        for (i, col) in self.columns.iter().enumerate() {
            if i > 0 { s.push_str(" | "); }
            s.push_str(&format!("{:width$}", col.name, width = widths[i]));
        }
        s.push('\n');
        for (i, w) in widths.iter().enumerate() {
            if i > 0 { s.push_str("-+-"); }
            s.push_str(&"-".repeat(*w));
        }
        s.push('\n');
        for r_idx in 0..num_rows {
            for c_idx in 0..num_cols {
                if c_idx > 0 { s.push_str(" | "); }
                let cell_idx = r_idx * num_cols + c_idx;
                s.push_str(&format!("{:width$}", self.rows[cell_idx].to_string(), width = widths[c_idx]));
            }
            s.push('\n');
        }
        s
    }
}

impl PartialEq for TableObj {
    fn eq(&self, other: &Self) -> bool {
        self.table_name == other.table_name && self.columns == other.columns && self.rows == other.rows && self.sql_where == other.sql_where && self.pending_op == other.pending_op
    }
}

impl Drop for TableObj {
    fn drop(&mut self) {
        for val in self.rows.iter() {
            unsafe { val.dec_ref(); }
        }
    }
}
