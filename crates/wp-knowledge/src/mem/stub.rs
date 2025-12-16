use crate::DBQuery;
use rusqlite::Params;
use wp_error::KnowledgeResult;
use wp_model_core::model::DataField;

#[derive(Debug, Clone)]
pub struct StubMDB {}

impl DBQuery for StubMDB {
    fn query(&self, _sql: &str) -> KnowledgeResult<Vec<Vec<DataField>>> {
        Ok(vec![])
    }

    fn query_row(&self, _sql: &str) -> KnowledgeResult<Vec<DataField>> {
        Ok(Vec::new())
    }

    fn query_cipher(&self, _table: &str) -> KnowledgeResult<Vec<String>> {
        Ok(vec![])
    }

    fn query_row_params<P: Params>(
        &self,
        _sql: &str,
        _params: P,
    ) -> KnowledgeResult<Vec<DataField>> {
        Ok(vec![])
    }

    fn query_row_tdos<P: Params>(
        &self,
        _sql: &str,
        _params: &[DataField; 2],
    ) -> KnowledgeResult<Vec<DataField>> {
        Ok(vec![])
    }
}
