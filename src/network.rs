use crate::{Error, check};
use neli::{
    consts::nl::*, consts::rtnl::*, consts::socket::*, err::*, nl::*, rtnl::*, socket::*, types::*,
};
use nix::unistd::getpid;
use std::os::fd::AsRawFd;

#[derive(Debug)]
struct IgnorePayload;

impl<'a> neli::FromBytesWithInput<'a> for IgnorePayload {
    type Input = usize;
    fn from_bytes_with_input(
        _buffer: &mut std::io::Cursor<&'a [u8]>,
        _input: Self::Input,
    ) -> Result<Self, DeError> {
        Ok(IgnorePayload)
    }
}

// TODO: learn Rust C FFI and look this up manually
// (neli::consts::rtnl::RtScope seems to have incorrect values)
const RT_SCOPE_HOST: u8 = 254;

fn read_nl_reply(netlink: &mut NlSocketHandle) -> Result<(), Error> {
    for message in netlink
        .iter::<'_, Nlmsg, IgnorePayload>(false /* wait until we receive a Done message */)
    {
        let message = check!(message, "error reading netlink reply: {}");
        if message.nl_type == Nlmsg::Error
            && let NlPayload::Ack(Nlmsgerr { error: 0, .. }) = message.nl_payload
        {
            return Ok(());
        } else {
            let e = format!("unknown reply from netlink:\n{message:?}");
            return Err(Error::InternalError(e));
        }
    }
    Ok(())
}

pub fn setup_network() -> Result<(), Error> {
    // partially reverse-engineered from bubblewrap, © 2016 Alexander Larsson, under LGPL 3.0
    // https://github.com/containers/bubblewrap/blob/bb7ac1348f98ee48f1e2e38bdf93abca2e4f6d06/network.c

    // also uses other references from netlink(7), rtnetlink(7), rtnetlink(4)

    let pid = getpid().as_raw() as u32;
    let mut sequence_number = 0;

    // create and bind socket to talk to the kernel
    let mut netlink = check!(
        NlSocketHandle::new(NlFamily::Route,),
        "error creating netlink socket: {}"
    );
    check!(
        netlink.bind(
            Some(pid),
            &[], // don't subscribe to any multicast groups
        ),
        "error binding netlink socket: {}"
    );

    // probably always returns 1
    let loopback = check!(
        netdevice::get_index(netlink.as_raw_fd(), "lo"),
        "error looking up lo: {}"
    );

    // add address 127.0.0.1/8
    let mut rtattrs = RtBuffer::new();
    for rta_type in [Ifa::Local, Ifa::Address] {
        rtattrs.push(Rtattr {
            rta_len: 8, /* = RTA_LENGTH(sizeof(struct in_addr)) */
            rta_type,
            // todo: does this need to be reversed?
            rta_payload: vec![127, 0, 0, 1].into(),
        });
    }
    let message = Ifaddrmsg {
        ifa_family: RtAddrFamily::Inet, // IPv4
        ifa_prefixlen: 8,               // netmask 255.0.0.0
        ifa_flags: IfaFFlags::new(&[IfaF::Permanent]),
        ifa_scope: RT_SCOPE_HOST, // scope determines distance to destination:
        // in this case the address is on the local host
        ifa_index: loopback,
        rtattrs,
    };
    let message = Nlmsghdr::new(
        None, // calculate size automatically
        Rtm::Newaddr,
        NlmFFlags::new(&[
            NlmF::Request, // we are sending a request
            NlmF::Create,  // if it doesn't exist, then create it
            NlmF::Excl,    // if it already exists, don't touch it
            NlmF::Ack,     // ask the kernel to acknowledge our request
        ]),
        Some(sequence_number),
        Some(pid),
        NlPayload::Payload(message),
    );
    check!(netlink.send(message), "error sending Ifaddrmsg: {}");
    read_nl_reply(&mut netlink)?;

    sequence_number += 1;

    // add interface
    let message = Ifinfomsg::new(
        RtAddrFamily::Unspecified,
        Arphrd::Loopback,
        loopback,
        IffFlags::new(&[
            Iff::Up, // set the interface running immediately
            Iff::Loopback,
        ]),
        IffFlags::from_bitmask(0xFFFFFFFF), // "ifi_change is reserved for future use and should always be set to 0xFFFFFFFF"
        RtBuffer::new(),                    // no routes
    );
    let message = Nlmsghdr::new(
        None, // calculate size automatically
        Rtm::Newlink,
        NlmFFlags::new(&[
            NlmF::Request, // we are sending a request
            NlmF::Ack,     // ask the kernel to acknowledge our request
        ]),
        Some(sequence_number),
        Some(pid),
        NlPayload::Payload(message),
    );
    check!(netlink.send(message), "error sending Ifaddrmsg: {}");
    read_nl_reply(&mut netlink)?;

    Ok(())
}
