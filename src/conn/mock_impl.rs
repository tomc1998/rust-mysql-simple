//! A module containing mock implementations for the Generic* traits found in the error::prelude
//! module. This is to allow users to easily mock connections, without having to re-implement the
//! mock connection in every project.
//!
//! These structs merely expose the trait functions as function pointers, allowing devs to provide
//! custom functionality and return values for queries, whilst also intercepting what was given to
//! the functions and recording the data to compare in the future for testing the behaviour without
//! an actual DB connection.

use std::hash::BuildHasherDefault as BldHshrDflt;
use std::collections::HashMap;
use fnv::FnvHasher;
use {Params, Value, Column, FromValueError, from_value, from_value_opt};
use super::{GenericConnection, GenericRow, GenericQueryResult, GenericStmt};
use error::Result as MyResult;
use prelude::*;
use std::sync::Arc;

/// A struct representing a type of request for a value from a row - i.e. get or take. These are
/// stored inside a MockRow, for developers to query and write tests based on the value.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
#[allow(dead_code)]
pub enum RowRequestType {
    Get,
    Take,
}

/// Mock implementation for a DB row.
///
/// The dev provides dummy values to return when various row methods are called, and can query the
/// mock row to see which values and columns have been requested and which haven't.
#[derive(Clone, Debug, PartialEq)]
pub struct MockRow {
    /// The list of values in this mock row.
    pub values: Vec<Option<Value>>,
    /// The list of columns in this mock row. These indices will match the values in the values
    /// vec.
    pub columns: Arc<Vec<Column>>,

    /// A list, in order, of the requests for columns made to this mock row.
    pub requests: Vec<(Column, RowRequestType)>,
}

impl GenericRow for MockRow {
    fn len(&self) -> usize {
        self.values.len()
    }

    fn as_ref(&self, index: usize) -> Option<&Value> {
        self.values.get(index).and_then(|x| x.as_ref())
    }

    fn get<T, I>(&mut self, index: I) -> Option<T>
    where
        I: ColumnIndex,
        T: FromValue,
    {
        index.idx(&*self.columns).and_then(|idx| {
            self.values.get(idx).and_then(|x| x.as_ref()).map(|x| {
                from_value::<T>(x.clone())
            })
        })
    }

    fn get_opt<T, I>(&mut self, index: I) -> Option<Result<T, FromValueError>>
    where
        I: ColumnIndex,
        T: FromValue,
    {
        index
            .idx(&*self.columns)
            .and_then(|idx| self.values.get(idx))
            .and_then(|x| x.as_ref())
            .and_then(|x| Some(from_value_opt::<T>(x.clone())))
    }

    fn take<T, I>(&mut self, index: I) -> Option<T>
    where
        I: ColumnIndex,
        T: FromValue,
    {
        index.idx(&*self.columns).and_then(|idx| {
            self.values.get_mut(idx).and_then(|x| x.take()).map(
                from_value::<T>,
            )
        })
    }

    fn take_opt<T, I>(&mut self, index: I) -> Option<Result<T, FromValueError>>
    where
        I: ColumnIndex,
        T: FromValue,
    {
        index
            .idx(&*self.columns)
            .and_then(|idx| self.values.get_mut(idx))
            .and_then(|x| x.take())
            .and_then(|x| Some(from_value_opt::<T>(x)))
    }

    fn unwrap(self) -> Vec<Value> {
        self.values
            .into_iter()
            .map(|x| {
                x.expect("Can't unwrap row if some of columns was taken")
            })
            .collect()
    }
}

pub struct MockStmt {
    pub params: Option<Vec<Column>>,
    pub columns: Option<Vec<Column>>,
    pub column_indexes: HashMap<String, usize, BldHshrDflt<FnvHasher>>,
    pub fn_execute: Option<Box<Fn(&str, Params) -> MyResult<MockQueryResult>>>,
    pub fn_first_exec: Option<Box<Fn(&str, Params) -> MyResult<Option<MockRow>>>>,
    pub query: String,
}

impl<'a> GenericStmt<'a> for MockStmt {
    type QueryResult = MockQueryResult;
    type Row = MockRow;

    fn params_ref(&self) -> Option<&[Column]> { self.params.as_ref().map(|v| &v[..]) }
    fn columns_ref(&self) -> Option<&[Column]> { self.columns.as_ref().map(|v| &v[..]) }
    fn column_index<T: AsRef<str>>(&self, name: T) -> Option<usize> {
        self.column_indexes.get(&name.as_ref().to_owned()).cloned()
    }
    fn execute<T: Into<Params>>(&'a mut self, params: T) -> MyResult<Self::QueryResult> {
        if self.fn_execute.is_some() { self.fn_execute.as_mut().unwrap()(&self.query, params.into()) }
        else { 
            panic!("Tried to call execute() on a mock statement without and implementation") 
        }
    }
    fn first_exec<T: Into<Params>>(&'a mut self, params: T) -> MyResult<Option<Self::Row>> {
        if self.fn_first_exec.is_some() { self.fn_first_exec.as_mut().unwrap()(&self.query, params.into()) }
        else { 
            panic!("Tried to call execute() on a mock statement without and implementation") 
        }
    }
}

pub struct MockQueryResult {
    pub affected_rows: u64,
    pub last_insert_id: u64,
    pub warnings: u16,
    pub info: Vec<u8>,
    pub column_indexes: HashMap<String, usize, BldHshrDflt<FnvHasher>>,
    pub columns: Vec<Column>,
    pub more_results_exists: bool,
    pub rows: Option<Vec<MyResult<MockRow>>>,
}

impl Iterator for MockQueryResult {
    type Item = MyResult<MockRow>;
    fn next(&mut self) -> Option<MyResult<MockRow>> { 
        if self.rows.is_none() { return None }
        self.rows.as_mut().unwrap().pop()
    } 
}

impl GenericQueryResult  for MockQueryResult {
    fn affected_rows(&self) -> u64 { self.affected_rows }
    fn last_insert_id(&self) -> u64 { self.last_insert_id }
    fn warnings(&self) -> u16 { self.warnings }
    fn info(&self) -> Vec<u8> { self.info.clone() }
    fn column_index<T: AsRef<str>>(&self, name: T) -> Option<usize> {
        self.column_indexes.get(&name.as_ref().to_owned()).cloned()
    }
    fn column_indexes(&self) -> HashMap<String, usize, BldHshrDflt<FnvHasher>> { self.column_indexes.clone() }
    fn columns_ref(&self) -> &[Column] { &self.columns[..] }
    fn more_results_exists(&self) -> bool { self.more_results_exists }
}

/// Mock implementation for a DB connection.
///
/// # Important
/// Not all the functions must be implemented. However, this struct provides no
/// default implementation. Therefore, if a mock implementation for a function
/// is not provided, but the function is called, the thread will panic.
///
/// # Examples
///
/// ```
/// // Initialise a mock connection with a mock prepare() method, then pass it
/// // into the register() function.
/// let mock_connection = MockConnection::new()
///     .with_fn_prepare(|q| {
///         log!("Query = {}", q);
///
///     });
/// ```
#[allow(dead_code)]
pub struct MockConnection {
    pub fn_query: Option<Box<Fn(&str) -> MyResult<MockQueryResult>>>,
    pub fn_first: Option<Box<Fn(&str) -> MyResult<Option<MockRow>>>>,
    pub fn_prepare: Option<Box<Fn(&str) -> MyResult<MockStmt>>>,
    pub fn_prep_exec: Option<Box<Fn(&str, Params) -> MyResult<MockQueryResult>>>,
    pub fn_first_exec: Option<Box<Fn(&str, Params) -> MyResult<Option<MockRow>>>>,
}

impl<'a> GenericConnection<'a> for MockConnection {
    type QueryResult = MockQueryResult;
    type Stmt = MockStmt;
    type Row = MockRow;

    fn query<Q: AsRef<str>>(&mut self, query: Q) -> MyResult<Self::QueryResult> {
        if self.fn_query.is_some() {
            self.fn_query.as_ref().unwrap()(query.as_ref())
        } else {
            panic!("Tried to call query() on mock connection with no implementation");
        }
    }

    fn first<Q: AsRef<str>>(&mut self, query: Q) -> MyResult<Option<Self::Row>> {
        if self.fn_first.is_some() {
            self.fn_first.as_ref().unwrap()(query.as_ref())
        } else {
            panic!("Tried to call first() on mock connection with no implementation");
        }
    }

    fn prepare<Q: AsRef<str>>(&mut self, query: Q) -> MyResult<Self::Stmt> {
        if self.fn_prepare.is_some() {
            self.fn_prepare.as_ref().unwrap()(query.as_ref())
        } else {
            panic!("Tried to call prepare() on mock connection with no implementation");
        }
    }

    fn prep_exec<Q, P>(&mut self, query: Q, params: P) -> MyResult<Self::QueryResult>
    where
        Q: AsRef<str>,
        P: Into<Params>,
    {
        if self.fn_prep_exec.is_some() {
            self.fn_prep_exec.as_ref().unwrap()(query.as_ref(), params.into())
        } else {
            panic!("Tried to call prep_exec() on mock connection with no implementation");
        }
    }

    fn first_exec<Q, P>(&mut self, query: Q, params: P) -> MyResult<Option<Self::Row>>
    where
        Q: AsRef<str>,
        P: Into<Params>,
    {
        if self.fn_first_exec.is_some() {
            self.fn_first_exec.as_ref().unwrap()(query.as_ref(), params.into())
        } else {
            panic!("Tried to call first_exec() on mock connection with no implementation");
        }
    }
}
