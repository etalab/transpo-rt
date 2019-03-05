use crate::actors::{DatasetActor, GetRealtimeDataset};
use crate::datasets::RealTimeDataset;
use crate::siri_lite::{
    general_message as gm, service_delivery::ServiceDelivery, shared::CommonDelivery, Siri,
    SiriResponse,
};
use crate::transit_realtime;
use actix::Addr;
use actix_web::{error, AsyncResponder, Error, Json, Query, Result, State};
use futures::future::Future;

// #[derive(Deserialize, Debug)]
// enum InfoChannel {
//     Perturbation,
//     Information,
//     Commercial,
// }

#[derive(Deserialize, Debug)]
pub struct Params {
    // info_channel_ref: Vec<InfoChannel>,
}

fn display_alert(_alert: &transit_realtime::Alert) -> bool {
    // TODO check active_period
    true
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

fn read_info_messages(feed: &transit_realtime::FeedMessage) -> Vec<gm::InfoMessage> {
    feed.entity
        .iter()
        .filter_map(|e| e.alert.as_ref())
        .filter(|a| display_alert(a))
        .map(|a| gm::InfoMessage {
            content: read_content(a),
            ..Default::default()
        })
        .collect()
}

fn general_message(_request: Params, rt_data: &RealTimeDataset) -> Result<SiriResponse> {
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
                    info_messages: read_info_messages(&feed),
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
