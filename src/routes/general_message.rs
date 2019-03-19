use crate::actors::{DatasetActor, GetRealtimeDataset};
use crate::datasets::RealTimeDataset;
use crate::siri_lite::{
    general_message as gm, service_delivery::ServiceDelivery, shared::CommonDelivery, Siri,
    SiriResponse,
};
use crate::transit_realtime;
use crate::utils;
use actix::Addr;
use actix_web::{error, AsyncResponder, Error, Json, Query, Result, State};
use futures::future::Future;

#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct Params {
    /// start_time is the datetime from which we want the next departures
    /// The default is the current time of the query
    request_timestamp: Option<crate::siri_lite::DateTime>,
}

fn get_max_validity(
    alert: &transit_realtime::Alert,
    timezone: chrono_tz::Tz,
) -> Option<crate::siri_lite::DateTime> {
    alert
        .active_period
        .iter()
        .filter_map(|p| utils::read_pbf_dt(p.end, timezone))
        .max()
        .map(crate::siri_lite::DateTime)
}

fn display_alert(
    alert: &transit_realtime::Alert,
    requested_dt: chrono::NaiveDateTime,
    timezone: chrono_tz::Tz,
) -> bool {
    alert.active_period.iter().any(|p| {
        utils::read_pbf_dt(p.start, timezone).map_or(true, |s| s <= requested_dt)
            && utils::read_pbf_dt(p.end, timezone).map_or(true, |e| requested_dt <= e)
    })
}

// we create one message by lang
fn get_msgs(
    ts: &Option<transit_realtime::TranslatedString>,
    msg_type: gm::MessageType,
) -> Vec<gm::Message> {
    ts.as_ref()
        .map(|translated_string| {
            translated_string
                .translation
                .iter()
                .map(|s| gm::Message {
                    message_type: Some(msg_type.clone()),
                    message_text: gm::NaturalLangString {
                        value: s.text.clone(),
                        lang: s.language.clone(),
                    },
                })
                .collect()
        })
        .unwrap_or_else(|| vec![])
}

fn read_content(alert: &transit_realtime::Alert) -> gm::GeneralMessageStructure {
    // use btreeset because there can be lots of dupplicates
    let mut line_ref = std::collections::BTreeSet::new();
    let mut sp_ref = std::collections::BTreeSet::new();
    let destination_ref = vec![]; // TODO, implement the destination
    for informed_entity in &alert.informed_entity {
        if let Some(s) = &informed_entity.stop_id {
            sp_ref.insert(s.clone());
        }
        if let Some(l) = &informed_entity.route_id {
            line_ref.insert(l.clone());
        }
    }

    gm::GeneralMessageStructure {
        line_ref: line_ref.into_iter().collect(),
        stop_point_ref: sp_ref.into_iter().collect(),
        destination_ref,
        // not sure about this, but we split the header/description as 2 different messages
        // a short and a long one
        message: get_msgs(&alert.header_text, gm::MessageType::shortMessage)
            .into_iter()
            .chain(get_msgs(&alert.description_text, gm::MessageType::longMessage).into_iter())
            .collect(),
    }
}

fn read_info_messages(
    feed: &transit_realtime::FeedMessage,
    requested_dt: chrono::NaiveDateTime,
    timezone: chrono_tz::Tz,
) -> Vec<gm::InfoMessage> {
    feed.entity
        .iter()
        .filter_map(|e| e.alert.as_ref())
        .filter(|a| display_alert(a, requested_dt, timezone))
        .map(|a| gm::InfoMessage {
            content: read_content(a),
            valid_until_time: get_max_validity(a, timezone),
            ..Default::default()
        })
        .collect()
}

fn general_message(request: Params, rt_data: &RealTimeDataset) -> Result<SiriResponse> {
    let requested_dt = request.request_timestamp.map(|d| d.0).unwrap_or_else(|| {
        chrono::Utc::now()
            .with_timezone(&rt_data.base_schedule_dataset.timezone)
            .naive_local()
    });
    // Note: we decode the gtfs at the query. if needed we can cache this, to parse it once
    use bytes::IntoBuf;
    use prost::Message;
    let feed = rt_data
        .gtfs_rt
        .as_ref()
        .ok_or_else(|| error::ErrorNotFound("no realtime data available"))
        .map(|rt| rt.data.clone())
        .and_then(|d| {
            transit_realtime::FeedMessage::decode(d.into_buf()).map_err(|e| {
                error::ErrorInternalServerError(format!(
                    "impossible to decode protobuf message: {}",
                    e
                ))
            })
        })?;

    Ok(SiriResponse {
        siri: Siri {
            service_delivery: Some(ServiceDelivery {
                producer_ref: None, // TODO take the id of the dataset ?
                general_message_delivery: vec![gm::GeneralMessageDelivery {
                    common: CommonDelivery::default(),
                    info_messages: read_info_messages(
                        &feed,
                        requested_dt,
                        rt_data.base_schedule_dataset.timezone,
                    ),
                    info_messages_cancellation: vec![],
                }],
                ..Default::default()
            }),
            ..Default::default()
        },
    })
}

pub fn general_message_query(
    (actor_addr, query): (State<Addr<DatasetActor>>, Query<Params>),
) -> Box<Future<Item = Json<SiriResponse>, Error = Error>> {
    actor_addr
        .send(GetRealtimeDataset)
        .map_err(Error::from)
        .and_then(|dataset| {
            dataset
                .and_then(|d| general_message(query.into_inner(), &*d))
                .map(Json)
        })
        .responder()
}
