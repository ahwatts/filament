use libc;
use std::ffi::CString;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::ptr;

pub fn lookup(hostname: &str) -> Result<Vec<IpAddr>, String> {
    let mut ips = vec!();
    let res = with_addrinfo(hostname, &mut |addr_info| {
        let sockaddr = unsafe {
            addr_info.ai_addr.as_ref()
        }.unwrap();

        match get_ip_addr(sockaddr) {
            Ok(ip) => {
                ips.push(ip)
            },
            Err(e) => {
                error!("Error with sockaddr (family: {:?} data: {:?}) for {:?}: {:?}",
                       sockaddr.sa_family, sockaddr.sa_data, hostname, e);
            }
        }
    });

    let rv = res.map(|_| ips);

    debug!("Looking up {:?}, got {:?}", hostname, rv);

    rv
}

#[allow(unused_assignments)]
fn with_addrinfo<F>(hostname: &str, callback: &mut F) -> Result<(), String>
    where F: FnMut(&libc::addrinfo)
{
    let addr_c_str = CString::new(hostname).unwrap();

    unsafe {
        let mut addr_info_list: *mut libc::addrinfo = ptr::null_mut();
        let result = libc::getaddrinfo(addr_c_str.as_ptr(), ptr::null(), ptr::null(), &mut addr_info_list);

        if result != 0 {
            return Err(format!("Error resolving {:?}: {}", hostname, result));
        } else {
            let mut addr_info_ptr = addr_info_list;
            while !addr_info_ptr.is_null() {
                let addr_info = addr_info_ptr.as_ref().unwrap();
                callback(addr_info);
                addr_info_ptr = (*addr_info_ptr).ai_next;
            }
            libc::freeaddrinfo(addr_info_ptr);
            addr_info_ptr = ptr::null_mut();
            Ok(())
        }
    }
}

fn get_ip_addr(sa: &libc::sockaddr) -> Result<IpAddr, String> {
    match sa.sa_family as i32 {
        libc::AF_INET => {
            let sa4: &libc::sockaddr_in = unsafe {
                ((sa as *const libc::sockaddr) as *const libc::sockaddr_in).as_ref()
            }.unwrap();
            Ok(IpAddr::V4(Ipv4Addr::from(u32::from_be(sa4.sin_addr.s_addr))))
        },
        libc::AF_INET6 => {
            let sa6: &libc::sockaddr_in6 = unsafe {
                ((sa as *const libc::sockaddr) as *const libc::sockaddr_in6).as_ref()
            }.unwrap();
            Ok(IpAddr::V6(Ipv6Addr::from(sa6.sin6_addr.s6_addr)))
        },
        _ => {
            Err(format!("Unknown address family: {}", sa.sa_family))
        },
    }
}
