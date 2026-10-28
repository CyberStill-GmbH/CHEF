//! Generación offline de nombres SRV candidatos a partir del catálogo del origen.
//! Estos nombres son sugerencias, no observaciones DNS ni autorización de consulta.

use crate::{ChefError, ChefResult, ErrorCode};

const COMMON_SRV_LABELS: [&str; 142] = [
    "_sipfederationtls._tcp",
    "_sip._tls",
    "_autodiscover._tcp",
    "_xmpp-server._tcp",
    "_sip._tcp",
    "_sips._tcp",
    "_caldavs._tcp",
    "_carddavs._tcp",
    "_caldav._tcp",
    "_carddav._tcp",
    "_sip._udp",
    "_imaps._tcp",
    "_submission._tcp",
    "_h323cs._tcp",
    "_h323ls._udp",
    "_xmpp-client._tcp",
    "_pop3s._tcp",
    "_jabber._tcp",
    "_imap._tcp",
    "_turn._udp",
    "_pop3._tcp",
    "_ldap._tcp",
    "_smtps._tcp",
    "_sipinternaltls._tcp",
    "_vlmcs._tcp",
    "_kerberos._udp",
    "_kerberos._tcp",
    "_kpasswd._udp",
    "_kpasswd._tcp",
    "_ntp._udp",
    "_gc._tcp",
    "_kerberos-master._udp",
    "_ldap._tcp.dc._msdcs",
    "_matrix._tcp",
    "_smtp._tcp",
    "_stun._udp",
    "_kerberos._tcp.dc._msdcs",
    "_ldap._tcp.gc._msdcs",
    "_kerberos-adm._tcp",
    "_ldap._tcp.pdc._msdcs",
    "_kerberos-master._tcp",
    "_http._tcp",
    "_h323rs._tcp",
    "_sipinternal._tcp",
    "_turn._tcp",
    "_stun._tcp",
    "_h323ls._tcp",
    "_x-puppet._tcp",
    "_h323cs._udp",
    "_stuns._tcp",
    "_jabber-client._tcp",
    "_x-puppet-ca._tcp",
    "_ts3._udp",
    "_minecraft._tcp",
    "_turns._tcp",
    "_ldaps._tcp",
    "_xmpps-client._tcp",
    "_https._tcp",
    "_ftp._tcp",
    "_xmpp-server._udp",
    "_xmpp-client._udp",
    "_jabber._udp",
    "_jabber-client._udp",
    "_xmpps-server._tcp",
    "_finger._tcp",
    "_stuns._udp",
    "_hkp._tcp",
    "_vlmcs._udp",
    "_turns._udp",
    "_tftp._udp",
    "_ssh._tcp",
    "_rtps._udp",
    "_mysqlsrv._tcp",
    "_hkps._tcp",
    "_h323be._udp",
    "_dns._tcp",
    "_wss._tcp",
    "_wpad._tcp",
    "_whois._tcp",
    "_webexconnect._tcp",
    "_webexconnects._tcp",
    "_vnc._tcp",
    "_test._tcp",
    "_telnet._tcp",
    "_telnets._tcp",
    "_teamspeak._tcp",
    "_svns._tcp",
    "_svcp._tcp",
    "_smb._tcp",
    "_sip-tls._tcp",
    "_sftp._tcp",
    "_secure-pop3._tcp",
    "_secure-imap._tcp",
    "_rtsp._tcp",
    "_rtps._tcp",
    "_rpc._tcp",
    "_rfb._tcp",
    "_raop._tcp",
    "_pstn._tcp",
    "_presence._tcp",
    "_pkixrep._tcp",
    "_pgprevokations._tcp",
    "_pgpkeys._tcp",
    "_ocsp._tcp",
    "_nntp._tcp",
    "_nfs._tcp",
    "_netbios-ssn._tcp",
    "_netbios-ns._tcp",
    "_netbios-dgm._tcp",
    "_mumble._tcp",
    "_msrpc._tcp",
    "_mqtts._tcp",
    "_minecraft._udp",
    "_iscsi._tcp",
    "_ircs._tcp",
    "_ipp._tcp",
    "_ipps._tcp",
    "_h323be._tcp",
    "_gits._tcp",
    "_ftps._tcp",
    "_ftpes._tcp",
    "_dnss._udp",
    "_dnss._tcp",
    "_diameter._tcp",
    "_crl._tcp",
    "_crls._tcp",
    "_cmp._tcp",
    "_certificates._tcp",
    "_aix._tcp",
    "_afpovertcp._tcp",
    "_collab-edge._tls",
    "_tcp",
    "_client._smtp",
    "_udp",
    "_tls",
    "_msdcs",
    "_gc._msdcs",
    "_ldaps._tcp.dc._msdcs",
    "_kerberos._tcp.kdc._msdcs",
    "_kerberos.tcp.dc._msdcs",
    "_imap",
    "_iax",
];

/// Genera candidatos de nombre SRV a partir del catálogo importado.
///
/// Solo acepta dominios raíz reconocidos por la sección ICANN de la PSL.
/// No consulta DNS, no verifica los candidatos y no modifica el scope CHEF.
pub fn common_dns_service_candidates(domain: &str) -> ChefResult<Vec<String>> {
    let Some(domain) = crate::host::canonical_domain(domain) else {
        return Err(invalid_domain());
    };
    let Some(registrable) = psl::domain(domain.as_bytes()) else {
        return Err(invalid_domain());
    };
    if registrable.suffix().typ() != Some(psl::Type::Icann)
        || registrable.as_bytes() != domain.as_bytes()
    {
        return Err(invalid_domain());
    }

    Ok(COMMON_SRV_LABELS
        .iter()
        .map(|label| format!("{label}.{domain}"))
        .filter(|candidate| candidate.len() <= 253)
        .collect())
}

fn invalid_domain() -> ChefError {
    ChefError::new(
        ErrorCode::InvalidInput,
        "La pista SRV requiere un dominio raíz canónico CHEF",
    )
}
