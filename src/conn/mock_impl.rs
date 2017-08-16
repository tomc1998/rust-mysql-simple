//! A module containing mock implementations for the Generic* traits found in the error::prelude
//! module. This is to allow users to easily mock connections, without having to re-implement the
//! mock connection in every project.
//!
//! These structs merely expose the trait functions as function pointers, allowing devs to provide
//! custom functionality and return values for queries, whilst also intercepting what was given to
//! the functions and recording the data to compare in the future for testing the behaviour without
//! an actual DB connection.

use error;
use {Params, Value, Column, FromValueError, from_value, from_value_opt};
use super::GenericConnection;
use prelude::*;
use std::sync::Arc;

/// A struct representing a type of request for a value from a row - i.e. get or take. These are
/// stored inside a MockRow, for developers to query and write tests based on the value.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum RowRequestType {
    Get,
    Take,
}

/// Mock implementation for a DB row.
///
/// The dev provides dummy values to return when various row methods are called, and can query the
/// mock row to see which values and columns have been requested and which haven't.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MockRow {
    /// The list of values in this mock row.
    values: Vec<Option<Value>>,
    /// The list of columns in this mock row. These indices will match the values in the values
    /// vec.
    columns: Arc<Vec<Column>>,

    /// A list, in order, of the requests for columns made to this mock row.
    requests: Vec<(Column, RowRequestType)>,
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


pub struct MockStmt;
pub struct MockQueryResult;

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
pub struct MockConnection {
    fn_query: Option<Box<Fn(&str) -> error::Result<MockQueryResult>>>,
    fn_first: Option<Box<Fn(&str) -> error::Result<Option<MockRow>>>>,
    fn_prepare: Option<Box<Fn(&str) -> error::Result<MockStmt>>>,
    fn_prep_exec: Option<Box<Fn(&str, Params) -> error::Result<MockQueryResult>>>,
    fn_first_exec: Option<Box<Fn(&str, Params) -> error::Result<Option<MockRow>>>>,
}

impl<'a> GenericConnection<'a> for MockConnection {
    type QueryResult = MockQueryResult;
    type Stmt = MockStmt;
    type Row = MockRow;

    fn query<Q: AsRef<str>>(&mut self, query: Q) -> error::Result<Self::QueryResult> {
        if self.fn_query.is_some() {
            self.fn_query.as_ref().unwrap()(query.as_ref())
        } else {
            panic!("Tried to call query() on mock connection with no implementation");
        }
    }

    fn first<Q: AsRef<str>>(&mut self, query: Q) -> error::Result<Option<Self::Row>> {
        if self.fn_first.is_some() {
            self.fn_first.as_ref().unwrap()(query.as_ref())
        } else {
            panic!("Tried to call first() on mock connection with no implementation");
        }
    }

    fn prepare<Q: AsRef<str>>(&mut self, query: Q) -> error::Result<Self::Stmt> {
        if self.fn_prepare.is_some() {
            self.fn_prepare.as_ref().unwrap()(query.as_ref())
        } else {
            panic!("Tried to call prepare() on mock connection with no implementation");
        }
    }

    fn prep_exec<Q, P>(&mut self, query: Q, params: P) -> error::Result<Self::QueryResult>
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

    fn first_exec<Q, P>(&mut self, query: Q, params: P) -> error::Result<Option<Self::Row>>
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
