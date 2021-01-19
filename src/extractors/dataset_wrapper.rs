use crate::actors::{DatasetActor, GetDataset, GetRealtimeDataset};
use crate::datasets::{Dataset, RealTimeDataset};
use actix::Addr;
use actix_web::{dev::Payload, web::Data, FromRequest, HttpRequest};
use futures::future::{err, FutureExt, LocalBoxFuture};
use std::sync::Arc;

/// This wrapper provides a convenient way to get a `Dataset` from an actix route.
/// It handles all the `DatasetActor` querying and the error handling.
///
/// ```
/// use transpo_rt::extractors::DatasetWrapper;
/// pub async fn a_route(dataset_wrapper: DatasetWrapper) -> actix_web::Result<()> {
///    let dataset = dataset_wrapper.get_dataset()?;
///    Ok(())
///}
/// ```
pub struct DatasetWrapper {
    dataset: Arc<Result<Dataset, anyhow::Error>>,
}

/// This wrapper provides a convenient way to get a `RealTimeDataset` from an actix route.
/// It handles all the `DatasetActor` querying and the error handling.
///
/// ```
/// use transpo_rt::extractors::RealTimeDatasetWrapper;
/// pub async fn a_route(rt_dataset_wrapper: RealTimeDatasetWrapper) -> actix_web::Result<()> {
///    let gtfs_rt = &rt_dataset_wrapper.gtfs_rt; // can be used as if it was a RealTimeDataset
///    let dataset = rt_dataset_wrapper.get_base_schedule_dataset()?;
///    Ok(())
///}
/// ```
pub struct RealTimeDatasetWrapper {
    realtime_dataset: Arc<RealTimeDataset>,
}

impl DatasetWrapper {
    pub fn get_dataset(&self) -> Result<&Dataset, actix_web::Error> {
        get_dataset(&self.dataset)
    }
}

impl std::ops::Deref for RealTimeDatasetWrapper {
    type Target = Arc<RealTimeDataset>;
    fn deref(&self) -> &Self::Target {
        &self.realtime_dataset
    }
}

impl RealTimeDatasetWrapper {
    pub fn get_base_schedule_dataset(&self) -> Result<&Dataset, actix_web::Error> {
        get_dataset(&self.realtime_dataset.base_schedule_dataset)
    }
}

fn get_dataset(d: &Arc<Result<Dataset, anyhow::Error>>) -> Result<&Dataset, actix_web::Error> {
    d.as_ref().as_ref().map_err(|e| {
        actix_web::error::ErrorBadGateway(format!(
            "theoretical dataset temporarily unavailable: {}",
            e
        ))
    })
}

impl FromRequest for DatasetWrapper {
    type Config = ();
    type Error = actix_web::Error;
    type Future = LocalBoxFuture<'static, Result<DatasetWrapper, actix_web::Error>>;

    fn from_request(req: &HttpRequest, _payload: &mut Payload) -> Self::Future {
        let dataset_actor = match req.app_data::<Data<Addr<DatasetActor>>>() {
            Some(d) => d,
            None => {
                return err(actix_web::error::ErrorInternalServerError(
                    "impossible to get data".to_string(),
                ))
                .boxed_local()
            }
        };

        dataset_actor
            .send(GetDataset)
            .map(|res| {
                res.map_err(|e| {
                    log::error!("error while querying actor for data: {:?}", e);
                    actix_web::error::ErrorInternalServerError("impossible to get data".to_string())
                })
                .map(|d| DatasetWrapper { dataset: d })
            })
            .boxed_local()
    }
}

impl FromRequest for RealTimeDatasetWrapper {
    type Config = ();
    type Error = actix_web::Error;
    type Future = LocalBoxFuture<'static, Result<RealTimeDatasetWrapper, actix_web::Error>>;

    fn from_request(req: &HttpRequest, _payload: &mut Payload) -> Self::Future {
        let dataset_actor = match req.app_data::<Data<Addr<DatasetActor>>>() {
            Some(d) => d,
            None => {
                return err(actix_web::error::ErrorInternalServerError(
                    "impossible to get real time data".to_string(),
                ))
                .boxed_local()
            }
        };

        dataset_actor
            .send(GetRealtimeDataset)
            .map(|res| {
                res.map_err(|e| {
                    log::error!("error while querying actor for data: {:?}", e);
                    actix_web::error::ErrorInternalServerError("impossible to get data".to_string())
                })
                .map(|d| RealTimeDatasetWrapper {
                    realtime_dataset: d,
                })
            })
            .boxed_local()
    }
}
