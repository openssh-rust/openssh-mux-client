macro_rules! def_constants {
    ( $name:ident, $val:literal ) => {
        pub const $name: u32 = $val;
    };
}

def_constants!(SSHMUX_VER, 4);

def_constants!(MUX_MSG_HELLO, 0x00000001);
def_constants!(MUX_C_NEW_SESSION, 0x10000002);
def_constants!(MUX_C_ALIVE_CHECK, 0x10000004);
def_constants!(MUX_C_OPEN_FWD, 0x10000006);
def_constants!(MUX_C_CLOSE_FWD, 0x10000007);
def_constants!(MUX_C_STOP_LISTENING, 0x10000009);
def_constants!(MUX_S_OK, 0x80000001);
def_constants!(MUX_S_PERMISSION_DENIED, 0x80000002);
def_constants!(MUX_S_FAILURE, 0x80000003);
def_constants!(MUX_S_EXIT_MESSAGE, 0x80000004);
def_constants!(MUX_S_ALIVE, 0x80000005);
def_constants!(MUX_S_SESSION_OPENED, 0x80000006);
def_constants!(MUX_S_REMOTE_PORT, 0x80000007);
def_constants!(MUX_S_TTY_ALLOC_FAIL, 0x80000008);

// MUX_C_CLOSE_FWD is not yet supported by openssh
// MUX_C_NEW_STDIO_FWD is not supported by this crate

//def_constants!(MUX_C_CLOSE_FWD,         0x10000007);
//def_constants!(MUX_C_NEW_STDIO_FWD,     0x10000008);

def_constants!(MUX_FWD_LOCAL, 1);
def_constants!(MUX_FWD_REMOTE, 2);
def_constants!(MUX_FWD_DYNAMIC, 3);
