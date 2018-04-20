//! SMIME implementation using CMS
//!
//! CMS (PKCS#7) is an encyption standard.  It allows signing and ecrypting data using
//! X.509 certificates.  The OpenSSL implementation of CMS is used in email encryption
//! generated from a `Vec` of bytes.  This `Vec` follows the smime protocol standards.
//! Data accepted by this module will be smime type `enveloped-data`.

use ffi;
use foreign_types::{ForeignType, ForeignTypeRef};
use std::ptr;

use bio::{MemBio, MemBioSlice};
use error::ErrorStack;
use pkey::{HasPrivate, PKeyRef};
use stack::Stack;
use x509::X509;
use {cvt, cvt_p};

foreign_type_and_impl_send_sync! {
    type CType = ffi::CMS_ContentInfo;
    fn drop = ffi::CMS_ContentInfo_free;

    /// High level CMS wrapper
    ///
    /// CMS supports nesting various types of data, including signatures, certificates,
    /// encrypted data, smime messages (encrypted email), and data digest.  The ContentInfo
    /// content type is the encapsulation of all those content types.  [`RFC 5652`] describes
    /// CMS and OpenSSL follows this RFC's implmentation.
    ///
    /// [`RFC 5652`]: https://tools.ietf.org/html/rfc5652#page-6
    pub struct CmsContentInfo;
    /// Reference to [`CMSContentInfo`]
    ///
    /// [`CMSContentInfo`]:struct.CmsContentInfo.html
    pub struct CmsContentInfoRef;
}

impl CmsContentInfoRef {
    /// Given the sender's private key, `pkey` and the recipient's certificiate, `cert`,
    /// decrypt the data in `self`.
    ///
    /// OpenSSL documentation at [`CMS_decrypt`]
    ///
    /// [`CMS_decrypt`]: https://www.openssl.org/docs/man1.1.0/crypto/CMS_decrypt.html
    pub fn decrypt<T>(&self, pkey: &PKeyRef<T>, cert: &X509) -> Result<Vec<u8>, ErrorStack>
    where
        T: HasPrivate,
    {
        unsafe {
            let pkey = pkey.as_ptr();
            let cert = cert.as_ptr();
            let out = MemBio::new()?;
            let flags: u32 = 0;

            cvt(ffi::CMS_decrypt(
                self.as_ptr(),
                pkey,
                cert,
                ptr::null_mut(),
                out.as_ptr(),
                flags.into(),
            ))?;

            Ok(out.get_buf().to_owned())
        }
    }
}

impl CmsContentInfo {
    /// Parses a smime formatted `vec` of bytes into a `CmsContentInfo`.
    ///
    /// OpenSSL documentation at [`SMIME_read_CMS`]
    ///
    /// [`SMIME_read_CMS`]: https://www.openssl.org/docs/man1.0.2/crypto/SMIME_read_CMS.html
    pub fn smime_read_cms(smime: &[u8]) -> Result<CmsContentInfo, ErrorStack> {
        unsafe {
            let bio = MemBioSlice::new(smime)?;

            let cms = cvt_p(ffi::SMIME_read_CMS(bio.as_ptr(), ptr::null_mut()))?;

            Ok(CmsContentInfo::from_ptr(cms))
        }
    }

    /// Given a signing cert `signcert`, private key `pkey`, an optional certificate stack `certs`,
    /// data `data` and flags `flags`, create a CmsContentInfo struct.
    ///
    /// OpenSSL documentation at [`CMS_sign`]
    ///
    /// [`CMS_sign`]: https://www.openssl.org/docs/manmaster/man3/CMS_sign.html
    pub fn sign<T: HasPrivate>(
        signcert: &X509,
        pkey: &PKeyRef<T>,
        certs: Option<&Stack<X509>>,
        data: &[u8],
        flags: u32,
    ) -> Result<CmsContentInfo, ErrorStack> {
        unsafe {
            let signcert = signcert.as_ptr();
            let pkey = pkey.as_ptr();
            let data_bio = MemBioSlice::new(data)?;
            let cms = cvt_p(ffi::CMS_sign(
                signcert,
                pkey,
                certs.unwrap_or(&Stack::<X509>::new()?).as_ptr(),
                data_bio.as_ptr(),
                flags,
            ))?;

            Ok(CmsContentInfo::from_ptr(cms))
        }
    }

    /// Serializes this CmsContentInfo using DER.
    ///
    /// OpenSSL documentation at [`i2d_CMS_ContentInfo`]
    ///
    /// [`i2d_CMS_ContentInfo`]: https://www.openssl.org/docs/man1.0.2/crypto/i2d_CMS_ContentInfo.html
    pub fn to_der(&mut self) -> Result<Vec<u8>, ErrorStack> {
        unsafe {
            let size = ffi::i2d_CMS_ContentInfo(self.as_ptr(), ptr::null_mut());
            let mut der = vec![0u8; size as usize];

            let raw_ptr = Box::into_raw(Box::new(der.as_mut_ptr()));
            ffi::i2d_CMS_ContentInfo(self.as_ptr(), raw_ptr);

            Box::from_raw(raw_ptr);
            Ok(der)
        }
    }
}
