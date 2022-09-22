use bytes::Bytes;

mod channel;

#[derive(Clone, Debug)]
pub(crate) enum Response {
    GlobalRequestFailure,

    GlobalRequestSuccess(
        /// Request specific data
        Bytes,
    ),
}
