# Authenticated encryption and key lifecycle

RWLang exposes a high-level user-data encryption primitive rather than a general cipher API.

```rwlang
let sealed = encryptUserData(keys.encryption, sensitiveValue);
let plain = decryptUserData(keys.encryption, sealed);
```

Security invariants:

- algorithm is fixed to AES-256-GCM;
- application code cannot choose cipher mode or nonce;
- encryption keys are exactly 256-bit key material in the runtime secret representation;
- the platform generates a fresh 96-bit nonce;
- ciphertext uses a versioned authenticated envelope (`rwenc1`);
- envelope version is authenticated as AAD;
- malformed/tampered ciphertext and wrong keys fail closed;
- plaintext size is bounded at this primitive boundary;
- encrypt accepts only active `EncryptionKey<UserData>`;
- decrypt accepts active or retiring keys;
- retired keys cannot be used.

This is a read-old/write-new rotation model: retiring keys remain available for migration reads without allowing new data to be encrypted under them.
