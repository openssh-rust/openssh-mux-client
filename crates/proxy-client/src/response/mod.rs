use bytes::Bytes;

#[derive(Clone, Debug)]
pub(crate) enum Response {
    GlobalRequestFailure,

    GlobalRequestSuccess(
        /// Request specific data
        Bytes,
    ),
}
