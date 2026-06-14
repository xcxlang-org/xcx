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
    pub rows: Vec<Vec<Value>>,
    pub sql_binding: Option<SqlBinding>,
    pub sql_where: Option<String>,
    pub pending_op: Option<MethodKind>,
}

impl Clone for TableObj {
    fn clone(&self) -> Self {
        for row in &self.rows {
            for val in row {
                unsafe { val.inc_ref(); }
            }
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
    pub fn to_json(&self) -> crate::vm::object::JsonVal {
        let mut rows = Vec::with_capacity(self.rows.len());
        for row in &self.rows {
            let mut obj = Vec::new();
            for (i, col) in self.columns.iter().enumerate() {
                if i < row.len() {
                    obj.push((std::sync::Arc::new(col.name.clone()), crate::vm::utils::json::value_to_json(&row[i])));
                }
            }
            rows.push(crate::vm::object::JsonVal::Object(std::sync::Arc::new(parking_lot::RwLock::new(obj))));
        }
        crate::vm::object::JsonVal::Array(std::sync::Arc::new(parking_lot::RwLock::new(rows)))
    }

    pub fn to_formatted_grid(&self) -> String {
        if self.columns.is_empty() { return String::new(); }
        let mut widths: Vec<usize> = self.columns.iter().map(|c| c.name.len()).collect();
        for row in &self.rows {
            for (i, v) in row.iter().enumerate() {
                if i < widths.len() {
                    widths[i] = widths[i].max(v.to_string().len());
                }
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
        for row in &self.rows {
            for (i, v) in row.iter().enumerate() {
                if i > 0 { s.push_str(" | "); }
                if i < widths.len() {
                    s.push_str(&format!("{:width$}", v.to_string(), width = widths[i]));
                }
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
        for row in self.rows.iter() {
            for val in row.iter() {
                unsafe { val.dec_ref(); }
            }
        }
    }
}
