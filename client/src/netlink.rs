use std::net::Ipv4Addr;

use futures::TryStreamExt;
use ipnetwork::IpNetwork;
use rtnetlink::Handle;

use crate::config::INTERFACE_NAME;
use crate::error::{Error, Result};
use crate::state::{AppState, Data};

async fn get_interface_index(handle: &Handle, name: &str) -> Result<u32> {
    let mut links = handle.link().get().match_name(name.to_string()).execute();

    if let Some(link) = links.try_next().await.map_err(Error::set_link_up)? {
        Ok(link.header.index)
    } else {
        Err(Error::get_interface(name.to_string()))
    }
}

pub async fn add_address(handle: &Handle, addr: Ipv4Addr, prefix_len: u8) -> Result<()> {
    let index = get_interface_index(handle, INTERFACE_NAME).await?;

    tracing::debug!(
        interface = INTERFACE_NAME,
        address = %addr,
        prefix_len,
        "adding address to interface"
    );

    match handle
        .address()
        .add(index, std::net::IpAddr::V4(addr), prefix_len)
        .execute()
        .await
    {
        Ok(()) => Ok(()),
        Err(rtnetlink::Error::NetlinkError(e)) if e.raw_code() == -libc::EEXIST => {
            tracing::debug!(
                interface = INTERFACE_NAME,
                address = %addr,
                "address already exists on interface"
            );
            Ok(())
        }
        Err(e) => Err(Error::set_address(e)),
    }
}

pub async fn add_routes(handle: &Handle, routes: &[IpNetwork]) -> Result<()> {
    let index = get_interface_index(handle, INTERFACE_NAME).await?;

    for route in routes {
        let result = match route {
            IpNetwork::V4(network) => {
                handle
                    .route()
                    .add()
                    .v4()
                    .destination_prefix(network.ip(), network.prefix())
                    .output_interface(index)
                    .execute()
                    .await
            }
            IpNetwork::V6(network) => {
                handle
                    .route()
                    .add()
                    .v6()
                    .destination_prefix(network.ip(), network.prefix())
                    .output_interface(index)
                    .execute()
                    .await
            }
        };

        match result {
            Ok(()) => {}
            Err(rtnetlink::Error::NetlinkError(e)) if e.raw_code() == -libc::EEXIST => {
                tracing::debug!(
                    destination = %route,
                    "route already exists"
                );
            }
            Err(e) => return Err(Error::add_route(e, route.to_string())),
        }
    }

    Ok(())
}

pub async fn set_link_up(handle: &Handle) -> Result<()> {
    let index = get_interface_index(handle, INTERFACE_NAME).await?;

    tracing::info!(interface = INTERFACE_NAME, "set interface up");

    handle
        .link()
        .set(index)
        .up()
        .execute()
        .await
        .map_err(Error::set_link_up)?;

    Ok(())
}

pub async fn delete_interface(handle: &Handle, name: &str) -> Result<()> {
    let index = get_interface_index(&handle, name).await?;

    handle
        .link()
        .del(index)
        .execute()
        .await
        .map_err(|e| Error::delete_interface(e, name))?;

    Ok(())
}

pub async fn connect() -> Result<(Handle, tokio::task::JoinHandle<()>)> {
    let (connection, handle, _) = rtnetlink::new_connection()
        .map_err(|e| Error::netlink(format!("failed to create netlink connection: {}", e)))?;

    let join_handle = tokio::spawn(connection);

    Ok((handle, join_handle))
}
