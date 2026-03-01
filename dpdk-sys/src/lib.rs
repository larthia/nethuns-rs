#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(dead_code)]
#![allow(non_upper_case_globals)]
#![allow(unsafe_op_in_unsafe_fn)]
#![allow(improper_ctypes)]

#[cfg(target_os = "linux")]
include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

#[cfg(target_os = "linux")]
#[repr(C, align(2))]
pub struct rte_ether_addr {
    _private: [u8; RTE_ETHER_ADDR_SIZE],
}

#[cfg(target_os = "linux")]
#[repr(C, align(2))]
pub struct rte_arp_ipv4 {
    _private: [u8; RTE_ARP_IPV4_SIZE],
}

#[cfg(target_os = "linux")]
#[repr(C, align(2))]
pub struct rte_arp_hdr {
    _private: [u8; RTE_ARP_HDR_SIZE],
}

#[cfg(target_os = "linux")]
#[repr(C)]
pub struct rte_l2tpv2_combined_msg_hdr {
    _private: [u8; RTE_L2TPV2_COMBINED_MSG_HDR_SIZE],
}

#[cfg(target_os = "linux")]
impl core::fmt::Debug for rte_gtp_psc_generic_hdr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("rte_gtp_psc_generic_hdr").finish()
    }
}