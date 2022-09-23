macro_rules! def_constants {
    ( $name:ident, $val:literal ) => {
        pub(crate) const $name: u8 = $val;
    };
}

def_constants!(SSH_MSG_GLOBAL_REQUEST, 80);
def_constants!(SSH_MSG_REQUEST_SUCCESS, 81);
def_constants!(SSH_MSG_REQUEST_FAILURE, 82);
def_constants!(SSH_MSG_CHANNEL_OPEN, 90);
def_constants!(SSH_MSG_CHANNEL_OPEN_CONFIRMATION, 91);
def_constants!(SSH_MSG_CHANNEL_OPEN_FAILURE, 92);
def_constants!(SSH_MSG_CHANNEL_WINDOW_ADJUST, 93);
def_constants!(SSH_MSG_CHANNEL_DATA, 94);
def_constants!(SSH_MSG_CHANNEL_EXTENDED_DATA, 95);
def_constants!(SSH_MSG_CHANNEL_EOF, 96);
def_constants!(SSH_MSG_CHANNEL_CLOSE, 97);
def_constants!(SSH_MSG_CHANNEL_REQUEST, 98);
def_constants!(SSH_MSG_CHANNEL_SUCCESS, 99);
def_constants!(SSH_MSG_CHANNEL_FAILURE, 100);

def_constants!(SSH_OPEN_ADMINISTRATIVELY_PROHIBITED, 1);
def_constants!(SSH_OPEN_CONNECT_FAILED, 2);
def_constants!(SSH_OPEN_UNKNOWN_CHANNEL_TYPE, 3);
def_constants!(SSH_OPEN_RESOURCE_SHORTAGE, 4);

def_constants!(SSH_EXTENDED_DATA_STDERR, 1);
