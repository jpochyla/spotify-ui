pub mod diffie_hellman;
pub mod shannon_codec;

use std::{
    convert::TryInto,
    io,
    io::{Read, Write},
    net::{TcpStream, ToSocketAddrs},
};

use byteorder::{ReadBytesExt, BE};
use hmac::{Hmac, Mac, NewMac};
use serde::{Deserialize, Serialize};
use sha1::Sha1;
use socks::Socks5Stream;
use url::Url;

use crate::{
    connection::{
        diffie_hellman::DHLocalKeys,
        shannon_codec::{ShannonDecoder, ShannonEncoder, ShannonMsg},
    },
    error::Error,
    protocol::authentication::AuthenticationType,
    util::{
        default_ureq_agent_builder, deserialize_protobuf, serialize_protobuf, NET_CONNECT_TIMEOUT,
        NET_IO_TIMEOUT,
    },
};

// Device ID used for authentication message.
const DEVICE_ID: &str = "Psst";

// URI of access-point resolve endpoint.
const AP_RESOLVE_ENDPOINT: &str = "http://apresolve.spotify.com";

// Access-point used in case the resolving fails.
const AP_FALLBACK: &str = "ap.spotify.com:443";

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(from = "SerializedCredentials")]
#[serde(into = "SerializedCredentials")]
pub struct Credentials {
    username: String,
    auth_data: Vec<u8>,
    auth_type: AuthenticationType,
}

impl Credentials {
    pub fn from_username_and_password(username: String, password: String) -> Self {
        Self {
            username,
            auth_type: AuthenticationType::AUTHENTICATION_USER_PASS,
            auth_data: password.into_bytes(),
        }
    }

    pub fn get_username(&self) -> String {
        self.username.clone()
    }
}

#[derive(Serialize, Deserialize)]
struct SerializedCredentials {
    username: String,
    auth_data: String,
    auth_type: i32,
}

impl From<SerializedCredentials> for Credentials {
    fn from(value: SerializedCredentials) -> Self {
        Self {
            username: value.username,
            auth_data: value.auth_data.into_bytes(),
            auth_type: value.auth_type.into(),
        }
    }
}

impl From<Credentials> for SerializedCredentials {
    fn from(value: Credentials) -> Self {
        Self {
            username: value.username,
            auth_data: String::from_utf8(value.auth_data)
                .expect("Invalid UTF-8 in serialized credentials"),
            auth_type: value.auth_type as _,
        }
    }
}

pub struct Transport {
    pub stream: TcpStream,
    pub encoder: ShannonEncoder<TcpStream>,
    pub decoder: ShannonDecoder<TcpStream>,
}

impl Transport {
    pub fn resolve_ap_with_fallback(proxy_url: Option<&str>) -> String {
        match Self::resolve_ap(proxy_url) {
            Ok(ap) => ap,
            Err(err) => {
                log::error!("using AP fallback, error while resolving: {:?}", err);
                AP_FALLBACK.into()
            }
        }
    }

    pub fn resolve_ap(proxy_url: Option<&str>) -> Result<String, Error> {
        #[derive(Clone, Debug, Deserialize)]
        struct APResolveData {
            ap_list: Vec<String>,
        }

        let agent = default_ureq_agent_builder(proxy_url)?.build();
        let data: APResolveData = agent.get(AP_RESOLVE_ENDPOINT).call()?.into_json()?;
        data.ap_list
            .into_iter()
            .next()
            .ok_or(Error::UnexpectedResponse)
    }

    pub fn connect(ap: &str, proxy_url: Option<&str>) -> Result<Self, Error> {
        log::trace!("connecting to: {:?} with proxy: {:?}", ap, proxy_url);
        let stream = if let Some(url) = proxy_url {
            Self::stream_through_proxy(ap, url)?
        } else {
            Self::stream_without_proxy(ap)?
        };
        if let Err(err) = stream.set_write_timeout(Some(NET_IO_TIMEOUT)) {
            log::warn!("failed to set TCP write timeout: {:?}", err);
        }
        log::trace!("connected");
        Self::exchange_keys(stream)
    }

    fn stream_without_proxy(ap: &str) -> Result<TcpStream, io::Error> {
        let mut last_err = None;
        for addr in ap.to_socket_addrs()? {
            match TcpStream::connect_timeout(&addr, NET_CONNECT_TIMEOUT) {
                Ok(stream) => {
                    return Ok(stream);
                }
                Err(err) => {
                    last_err.replace(err);
                }
            }
        }
        Err(last_err.unwrap_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "could not resolve to any addresses",
            )
        }))
    }

    fn stream_through_proxy(ap: &str, url: &str) -> Result<TcpStream, Error> {
        match Url::parse(url) {
            Ok(url) if url.scheme() == "socks" || url.scheme() == "socks5" => {
                // Currently we only support SOCKS5 proxies.
                Self::stream_through_socks5_proxy(ap, &url)
            }
            _ => {
                // Proxy URL failed to parse or has unsupported scheme.
                Err(Error::ProxyUrlInvalid)
            }
        }
    }

    fn stream_through_socks5_proxy(ap: &str, url: &Url) -> Result<TcpStream, Error> {
        let addrs = url.socket_addrs(|| None)?;
        let username = url.username();
        let password = url.password().unwrap_or("");
        // TODO: `socks` crate does not support connection timeouts.
        let proxy = if username.is_empty() {
            Socks5Stream::connect(&addrs[..], ap)?
        } else {
            Socks5Stream::connect_with_password(&addrs[..], ap, username, password)?
        };
        Ok(proxy.into_inner())
    }

    pub fn exchange_keys(mut stream: TcpStream) -> Result<Self, Error> {
        use crate::protocol::keyexchange::APResponseMessage;

        let local_keys = DHLocalKeys::random();

        // Start by sending the hello message with our public key and nonce.
        log::trace!("sending client hello");
        let client_nonce: [u8; 16] = rand::random();
        let hello = client_hello(local_keys.public_key(), client_nonce.into());
        let hello_packet = make_packet(&[0, 4], &hello);
        stream.write_all(&hello_packet)?;
        log::trace!("sent client hello");

        // Wait for the response packet with the remote public key.  Note that we are
        // keeping both the hello packet and the response packet for later (they get
        // hashed together with the shared secret to make a key pair).
        log::trace!("waiting for AP response");
        let apresp_packet = read_packet(&mut stream)?;
        let apresp: APResponseMessage = deserialize_protobuf(&apresp_packet[4..])?;
        log::trace!("received AP response");

        // Compute the challenge response and the sending/receiving keys.
        let remote_key = &apresp
            .challenge
            .expect("Missing data")
            .login_crypto_challenge
            .diffie_hellman
            .expect("Missing data")
            .gs;
        let (challenge, send_key, recv_key) = compute_keys(
            &local_keys.shared_secret(remote_key),
            &hello_packet,
            &apresp_packet,
        );

        // Respond with the computed HMAC and finish the handshake.
        log::trace!("sending client response");
        let response = client_response_plaintext(challenge);
        let response_packet = make_packet(&[], &response);
        stream.write_all(&response_packet)?;
        log::trace!("sent client response");

        // Use the derived keys to make a codec, wrapping the TCP stream.
        let encoder = ShannonEncoder::new(stream.try_clone()?, &send_key);
        let decoder = ShannonDecoder::new(stream.try_clone()?, &recv_key);

        Ok(Self {
            stream,
            encoder,
            decoder,
        })
    }

    pub fn authenticate(&mut self, credentials: Credentials) -> Result<Credentials, Error> {
        use crate::protocol::{authentication::APWelcome, keyexchange::APLoginFailed};

        // Send a login request with the client credentials.
        let request = client_response_encrypted(credentials);
        self.encoder.encode(request)?;

        // Expect an immediate response with the authentication result.
        let response = self.decoder.decode()?;

        match response.cmd {
            ShannonMsg::AP_WELCOME => {
                let welcome_data: APWelcome =
                    deserialize_protobuf(&response.payload).expect("Missing data");
                Ok(Credentials {
                    username: welcome_data.canonical_username,
                    auth_data: welcome_data.reusable_auth_credentials,
                    auth_type: welcome_data.reusable_auth_credentials_type,
                })
            }
            ShannonMsg::AUTH_FAILURE => {
                let error_data: APLoginFailed =
                    deserialize_protobuf(&response.payload).expect("Missing data");
                Err(Error::AuthFailed {
                    code: error_data.error_code as _,
                })
            }
            _ => {
                unreachable!("unexpected message");
            }
        }
    }
}

fn read_packet(stream: &mut TcpStream) -> io::Result<Vec<u8>> {
    let size = stream.read_u32::<BE>()?;
    let mut buf = vec![0_u8; size as usize];
    let (size_buf, data_buf) = buf.split_at_mut(4);
    size_buf.copy_from_slice(&size.to_be_bytes());
    stream.read_exact(data_buf)?;
    Ok(buf)
}

fn make_packet(prefix: &[u8], data: &[u8]) -> Vec<u8> {
    let size = prefix.len() + 4 + data.len();
    let mut buf = Vec::with_capacity(size);
    let size_u32: u32 = size.try_into().unwrap();
    buf.extend(prefix);
    buf.extend(size_u32.to_be_bytes());
    buf.extend(data);
    buf
}

fn client_hello(public_key: Vec<u8>, nonce: Vec<u8>) -> Vec<u8> {
    use crate::protocol::keyexchange::*;

    let hello = ClientHello {
        build_info: BuildInfo {
            platform: Platform::PLATFORM_LINUX_X86,
            product: Product::PRODUCT_PARTNER,
            product_flags: vec![],
            version: 109_800_078,
        },
        cryptosuites_supported: vec![Cryptosuite::CRYPTO_SUITE_SHANNON],
        fingerprints_supported: vec![],
        powschemes_supported: vec![],
        login_crypto_hello: LoginCryptoHelloUnion {
            diffie_hellman: Some(LoginCryptoDiffieHellmanHello {
                gc: public_key,
                server_keys_known: 1,
            }),
        },
        client_nonce: nonce,
        padding: Some(vec![0x1e]),
        feature_set: None,
    };

    serialize_protobuf(&hello).expect("Failed to serialize")
}

fn client_response_plaintext(challenge: Vec<u8>) -> Vec<u8> {
    use crate::protocol::keyexchange::*;

    let response = ClientResponsePlaintext {
        login_crypto_response: LoginCryptoResponseUnion {
            diffie_hellman: Some(LoginCryptoDiffieHellmanResponse { hmac: challenge }),
        },
        pow_response: PoWResponseUnion::default(),
        crypto_response: CryptoResponseUnion::default(),
    };

    serialize_protobuf(&response).expect("Failed to serialize")
}

fn compute_keys(
    shared_secret: &[u8],
    hello_packet: &[u8],
    apresp_packet: &[u8],
) -> (Vec<u8>, Vec<u8>, Vec<u8>) {
    let mut data = Vec::with_capacity(5 * 20);
    for i in 1..6 {
        let mut mac: Hmac<Sha1> =
            Hmac::new_from_slice(shared_secret).expect("HMAC can take key of any size");
        mac.update(hello_packet);
        mac.update(apresp_packet);
        mac.update(&[i]);
        data.extend(mac.finalize().into_bytes());
    }
    let mut mac: Hmac<Sha1> =
        Hmac::new_from_slice(&data[..20]).expect("HMAC can take key of any size");
    mac.update(hello_packet);
    mac.update(apresp_packet);
    let digest = mac.finalize().into_bytes();

    (
        (&*digest).to_vec(),
        (&data[20..52]).to_vec(),
        (&data[52..84]).to_vec(),
    )
}

fn client_response_encrypted(credentials: Credentials) -> ShannonMsg {
    use crate::protocol::authentication::{ClientResponseEncrypted, LoginCredentials, SystemInfo};

    let response = ClientResponseEncrypted {
        login_credentials: LoginCredentials {
            username: Some(credentials.username),
            auth_data: Some(credentials.auth_data),
            typ: credentials.auth_type,
        },
        system_info: SystemInfo {
            device_id: Some(DEVICE_ID.to_string()),
            ..SystemInfo::default()
        },
        ..ClientResponseEncrypted::default()
    };

    let buf = serialize_protobuf(&response).expect("Failed to serialize");
    ShannonMsg::new(ShannonMsg::LOGIN, buf)
}
