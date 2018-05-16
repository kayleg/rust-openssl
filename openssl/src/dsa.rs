//! Digital Signatures
//!
//! DSA ensures a message originated from a known sender, and was not modified.
//! DSA uses asymetrical keys and an algorithm to output a signature of the message
//! using the private key that can be validated with the public key but not be generated
//! without the private key.

use ffi;
use foreign_types::ForeignTypeRef;
use libc::{c_char, c_int, c_void};
use std::fmt;
use std::ptr;

use bio::MemBioSlice;
use bn::BigNumRef;
use error::ErrorStack;
use util::{invoke_passwd_cb_old, CallbackState};
use {cvt, cvt_p};

foreign_type_and_impl_send_sync! {
    type CType = ffi::DSA;
    fn drop = ffi::DSA_free;

    /// Object representing DSA keys.
    ///
    /// A DSA object contains the parameters p, q, and g.  There is a private
    /// and public key.  The values p, g, and q are:
    ///
    /// * `p`: DSA prime parameter
    /// * `q`: DSA sub-prime parameter
    /// * `g`: DSA base parameter
    ///
    /// These values are used to calculate a pair of asymetrical keys used for
    /// signing.
    ///
    /// OpenSSL documentation at [`DSA_new`]
    ///
    /// [`DSA_new`]: https://www.openssl.org/docs/man1.1.0/crypto/DSA_new.html
    ///
    /// # Examples
    ///
    /// ```
    /// use openssl::dsa::Dsa;
    /// use openssl::error::ErrorStack;
    /// fn create_dsa() -> Result< Dsa, ErrorStack > {
    ///     let sign = Dsa::generate(2048)?;
    ///     Ok(sign)
    /// }
    /// # fn main() {
    /// #    create_dsa();
    /// # }
    /// ```
    pub struct Dsa;
    /// Reference to [`Dsa`].
    ///
    /// [`Dsa`]: struct.Dsa.html
    pub struct DsaRef;
}

impl DsaRef {
    private_key_to_pem!(ffi::PEM_write_bio_DSAPrivateKey);
    public_key_to_pem!(ffi::PEM_write_bio_DSA_PUBKEY);

    private_key_to_der!(ffi::i2d_DSAPrivateKey);
    public_key_to_der!(ffi::i2d_DSAPublicKey);

    /// Returns the maximum size of the signature output by `self` in bytes.  Returns
    /// None if the keys are uninitialized.
    ///
    /// OpenSSL documentation at [`DSA_size`]
    ///
    /// [`DSA_size`]: https://www.openssl.org/docs/man1.1.0/crypto/DSA_size.html
    // FIXME should return u32
    pub fn size(&self) -> Option<u32> {
        if self.q().is_some() {
            unsafe { Some(ffi::DSA_size(self.as_ptr()) as u32) }
        } else {
            None
        }
    }

    /// Returns the DSA prime parameter of `self`.
    pub fn p(&self) -> Option<&BigNumRef> {
        unsafe {
            let p = compat::pqg(self.as_ptr())[0];
            if p.is_null() {
                None
            } else {
                Some(BigNumRef::from_ptr(p as *mut _))
            }
        }
    }

    /// Returns the DSA sub-prime parameter of `self`.
    pub fn q(&self) -> Option<&BigNumRef> {
        unsafe {
            let q = compat::pqg(self.as_ptr())[1];
            if q.is_null() {
                None
            } else {
                Some(BigNumRef::from_ptr(q as *mut _))
            }
        }
    }

    /// Returns the DSA base parameter of `self`.
    pub fn g(&self) -> Option<&BigNumRef> {
        unsafe {
            let g = compat::pqg(self.as_ptr())[2];
            if g.is_null() {
                None
            } else {
                Some(BigNumRef::from_ptr(g as *mut _))
            }
        }
    }

    /// Returns whether the DSA includes a public key, used to confirm the authenticity
    /// of the message.
    pub fn has_public_key(&self) -> bool {
        unsafe { !compat::keys(self.as_ptr())[0].is_null() }
    }

    /// Returns whether the DSA includes a private key, used to prove the authenticity
    /// of a message.
    pub fn has_private_key(&self) -> bool {
        unsafe { !compat::keys(self.as_ptr())[1].is_null() }
    }
}

impl Dsa {
    /// Generate a DSA key pair.
    ///
    /// Calls [`DSA_generate_parameters_ex`] to populate the `p`, `g`, and `q` values.
    /// These values are used to generate the key pair with [`DSA_generate_key`].
    ///
    /// The `bits` parameter coresponds to the length of the prime `p`.
    ///
    /// [`DSA_generate_parameters_ex`]: https://www.openssl.org/docs/man1.1.0/crypto/DSA_generate_parameters_ex.html
    /// [`DSA_generate_key`]: https://www.openssl.org/docs/man1.1.0/crypto/DSA_generate_key.html
    pub fn generate(bits: u32) -> Result<Dsa, ErrorStack> {
        ffi::init();
        unsafe {
            let dsa = Dsa(cvt_p(ffi::DSA_new())?);
            cvt(ffi::DSA_generate_parameters_ex(
                dsa.0,
                bits as c_int,
                ptr::null(),
                0,
                ptr::null_mut(),
                ptr::null_mut(),
                ptr::null_mut(),
            ))?;
            cvt(ffi::DSA_generate_key(dsa.0))?;
            Ok(dsa)
        }
    }

    private_key_from_pem!(Dsa, ffi::PEM_read_bio_DSAPrivateKey);
    private_key_from_der!(Dsa, ffi::d2i_DSAPrivateKey);
    public_key_from_pem!(Dsa, ffi::PEM_read_bio_DSA_PUBKEY);
    public_key_from_der!(Dsa, ffi::d2i_DSAPublicKey);

    #[deprecated(since = "0.9.2", note = "use private_key_from_pem_callback")]
    pub fn private_key_from_pem_cb<F>(buf: &[u8], pass_cb: F) -> Result<Dsa, ErrorStack>
    where
        F: FnOnce(&mut [c_char]) -> usize,
    {
        ffi::init();
        let mut cb = CallbackState::new(pass_cb);
        let mem_bio = MemBioSlice::new(buf)?;

        unsafe {
            let cb_ptr = &mut cb as *mut _ as *mut c_void;
            let dsa = cvt_p(ffi::PEM_read_bio_DSAPrivateKey(
                mem_bio.as_ptr(),
                ptr::null_mut(),
                Some(invoke_passwd_cb_old::<F>),
                cb_ptr,
            ))?;
            Ok(Dsa(dsa))
        }
    }
}

impl fmt::Debug for Dsa {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "DSA")
    }
}

#[cfg(ossl110)]
mod compat {
    use ffi::{self, BIGNUM, DSA};
    use std::ptr;

    pub unsafe fn pqg(d: *const DSA) -> [*const BIGNUM; 3] {
        let (mut p, mut q, mut g) = (ptr::null(), ptr::null(), ptr::null());
        ffi::DSA_get0_pqg(d, &mut p, &mut q, &mut g);
        [p, q, g]
    }

    pub unsafe fn keys(d: *const DSA) -> [*const BIGNUM; 2] {
        let (mut pub_key, mut priv_key) = (ptr::null(), ptr::null());
        ffi::DSA_get0_key(d, &mut pub_key, &mut priv_key);
        [pub_key, priv_key]
    }
}

#[cfg(ossl10x)]
mod compat {
    use ffi::{BIGNUM, DSA};

    pub unsafe fn pqg(d: *const DSA) -> [*const BIGNUM; 3] {
        [(*d).p, (*d).q, (*d).g]
    }

    pub unsafe fn keys(d: *const DSA) -> [*const BIGNUM; 2] {
        [(*d).pub_key, (*d).priv_key]
    }
}

#[cfg(test)]
mod test {
    use symm::Cipher;

    use super::*;

    #[test]
    pub fn test_generate() {
        Dsa::generate(1024).unwrap();
    }

    #[test]
    pub fn test_password() {
        let key = include_bytes!("../test/dsa-encrypted.pem");
        Dsa::private_key_from_pem_passphrase(key, b"mypass").unwrap();
    }

    #[test]
    fn test_to_password() {
        let key = Dsa::generate(2048).unwrap();
        let pem = key.private_key_to_pem_passphrase(Cipher::aes_128_cbc(), b"foobar")
            .unwrap();
        Dsa::private_key_from_pem_passphrase(&pem, b"foobar").unwrap();
        assert!(Dsa::private_key_from_pem_passphrase(&pem, b"fizzbuzz").is_err());
    }

    #[test]
    pub fn test_password_callback() {
        let mut password_queried = false;
        let key = include_bytes!("../test/dsa-encrypted.pem");
        Dsa::private_key_from_pem_callback(key, |password| {
            password_queried = true;
            password[..6].copy_from_slice(b"mypass");
            Ok(6)
        }).unwrap();

        assert!(password_queried);
    }
}
