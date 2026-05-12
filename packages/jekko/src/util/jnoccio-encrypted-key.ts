export type JnoccioEncryptedGitCryptKeyEnvelope = {
  version: 1
  kdf: "scrypt"
  cipher: "aes-256-gcm"
  salt: string
  iv: string
  tag: string
  ciphertext: string
  params: {
    N: number
    r: number
    p: number
    maxmem: number
  }
  aad: "jnoccio-fusion-git-crypt-key-v1"
}

export const JNOCCIO_ENCRYPTED_GIT_CRYPT_KEY = {
  "version": 1,
  "kdf": "scrypt",
  "cipher": "aes-256-gcm",
  "salt": "p7kKIm3yOgV1ztwDz-zh-Q",
  "iv": "LSqJhuHtPp4QrGXW",
  "tag": "RRI9y87haWJ39q4TuMZ-eA",
  "ciphertext": "rLX1c53q2pGv7yNokmAXN-Tl5ObbnhrO8-V5vDoiQeIUTEErk7eRlEPhBDfnlyJBn1J-70hQ4jMj5o7xeYnP5mk3LVZP_Dm_S8uwP-T13NKWPLICpeGj2bkf9DMJzw58o655S3AwNw5bMUctJt2IRm0cturz09W30kQ-TcocGBjrvRE-NwTewLgRGLBKkIfrNNdEFA",
  "params": {
    "N": 262144,
    "r": 8,
    "p": 1,
    "maxmem": 536870912
  },
  "aad": "jnoccio-fusion-git-crypt-key-v1"
} as const satisfies JnoccioEncryptedGitCryptKeyEnvelope
