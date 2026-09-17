// WI067 Checkpoint E (GH-004): Android Keystore-backed protected
// credential storage.
//
// This plugin never stores an OAuth token string directly in Android
// Keystore -- Keystore owns cryptographic key material, not arbitrary
// application secret blobs. Instead it generates a non-exportable
// AES-256-GCM key *inside* AndroidKeyStore (alias `KEY_ALIAS` below),
// uses it for authenticated encryption of the caller-supplied secret, and
// writes only the resulting versioned envelope (never the key) to this
// app's private SharedPreferences file. `putCredential`/`getCredential`/
// `deleteCredential` are the entire surface; there is no command that
// exports the raw key, and no command reachable from the web frontend at
// all (`repopact-mobile-credential`'s Rust side registers zero
// `#[tauri::command]`s -- see that crate's `lib.rs`).
//
// Structured after this repository's own `SafAcquisitionPlugin.kt`
// (`repopact-mobile-saf`): a narrow `@TauriPlugin`/`@Command` bridge that
// performs no business logic beyond what the Android platform forces it
// to own here (Keystore/Cipher/SharedPreferences mechanics).

package com.forgewirelabs.repopact.mobilecredential

import android.content.Context
import android.content.SharedPreferences
import android.security.keystore.KeyGenParameterSpec
import android.security.keystore.KeyPermanentlyInvalidatedException
import android.security.keystore.KeyProperties
import android.util.Base64
import app.tauri.Logger
import app.tauri.annotation.Command
import app.tauri.annotation.InvokeArg
import app.tauri.annotation.TauriPlugin
import app.tauri.plugin.Invoke
import app.tauri.plugin.JSObject
import app.tauri.plugin.Plugin
import java.io.IOException
import java.security.GeneralSecurityException
import java.security.KeyStore
import java.security.UnrecoverableKeyException
import javax.crypto.AEADBadTagException
import javax.crypto.Cipher
import javax.crypto.KeyGenerator
import javax.crypto.SecretKey
import javax.crypto.spec.GCMParameterSpec

/** Dedicated alias scoped to this application and this feature -- never
 * derived from client id, device id, username, package name, a
 * hard-coded password, or any GitHub data (Checkpoint E brief). */
private const val KEY_ALIAS = "com.forgewirelabs.repopact.remote-provider.v1"

private const val ANDROID_KEYSTORE = "AndroidKeyStore"
private const val TRANSFORMATION = "AES/GCM/NoPadding"
private const val GCM_TAG_LENGTH_BITS = 128
private const val GCM_IV_LENGTH_BYTES = 12
private const val ENVELOPE_VERSION = "1"

/** App-private SharedPreferences file (Context.MODE_PRIVATE): readable
 * only by this application's own UID, never world-readable, never routed
 * through SAF, never touching external storage. Holds ciphertext-only
 * envelopes, keyed by an opaque SHA-256 hash the Rust side computes from
 * credential identity -- never a raw provider/connection id, and never
 * access-token text. */
private const val PREFS_NAME = "repopact_credential_store"

private class EnvelopeFormatException(message: String) : Exception(message)

@InvokeArg
class PutCredentialArgs {
  lateinit var storageKey: String
  lateinit var secret: String
}

@InvokeArg
class GetCredentialArgs {
  lateinit var storageKey: String
}

@InvokeArg
class DeleteCredentialArgs {
  lateinit var storageKey: String
}

@TauriPlugin
class RepopactCredentialPlugin(private val activity: android.app.Activity) : Plugin(activity) {

  private fun prefs(): SharedPreferences =
    activity.applicationContext.getSharedPreferences(PREFS_NAME, Context.MODE_PRIVATE)

  private fun keyStore(): KeyStore {
    val keyStore = KeyStore.getInstance(ANDROID_KEYSTORE)
    keyStore.load(null)
    return keyStore
  }

  /** Used only by `putCredential`: generates the key the first time it is
   * needed. Never exportable (`KeyGenParameterSpec` with no
   * `setUserConfirmationRequired`/export path exists for an
   * AndroidKeyStore-backed `SecretKey` in the first place -- the key
   * material itself never leaves the keystore process). */
  private fun getOrCreateSecretKey(): SecretKey {
    val existing = getExistingSecretKey()
    if (existing != null) return existing
    val generator = KeyGenerator.getInstance(KeyProperties.KEY_ALGORITHM_AES, ANDROID_KEYSTORE)
    val spec = KeyGenParameterSpec.Builder(
      KEY_ALIAS,
      KeyProperties.PURPOSE_ENCRYPT or KeyProperties.PURPOSE_DECRYPT
    )
      .setBlockModes(KeyProperties.BLOCK_MODE_GCM)
      .setEncryptionPaddings(KeyProperties.ENCRYPTION_PADDING_NONE)
      .setKeySize(256)
      .setRandomizedEncryptionRequired(true)
      .build()
    generator.init(spec)
    return generator.generateKey()
  }

  /** Used by `getCredential`/`deleteCredential`'s decrypt path: returns
   * null rather than creating a key, so a genuinely missing/deleted key is
   * reported as `key_unavailable` rather than masked by transparently
   * minting a fresh key that can never decrypt the existing envelope
   * anyway. */
  private fun getExistingSecretKey(): SecretKey? {
    val entry = keyStore().getKey(KEY_ALIAS, null) ?: return null
    return entry as? SecretKey
  }

  private fun encrypt(secret: String): String {
    val key = getOrCreateSecretKey()
    val cipher = Cipher.getInstance(TRANSFORMATION)
    cipher.init(Cipher.ENCRYPT_MODE, key)
    val iv = cipher.iv
    val ciphertext = cipher.doFinal(secret.toByteArray(Charsets.UTF_8))
    val ivPart = Base64.encodeToString(iv, Base64.NO_WRAP)
    val ciphertextPart = Base64.encodeToString(ciphertext, Base64.NO_WRAP)
    return "$ENVELOPE_VERSION:$ivPart:$ciphertextPart"
  }

  /** Never returns a value on any parse/format problem -- a malformed or
   * truncated envelope fails typed (`EnvelopeFormatException`), never
   * silently treated as decryptable garbage. */
  private fun decrypt(envelope: String, key: SecretKey): String {
    val parts = envelope.split(":")
    if (parts.size != 3 || parts[0] != ENVELOPE_VERSION) {
      throw EnvelopeFormatException("unrecognized envelope version or shape")
    }
    val iv = try {
      Base64.decode(parts[1], Base64.NO_WRAP)
    } catch (ex: IllegalArgumentException) {
      throw EnvelopeFormatException("malformed nonce encoding")
    }
    if (iv.size != GCM_IV_LENGTH_BYTES) {
      throw EnvelopeFormatException("malformed nonce length")
    }
    val ciphertext = try {
      Base64.decode(parts[2], Base64.NO_WRAP)
    } catch (ex: IllegalArgumentException) {
      throw EnvelopeFormatException("malformed ciphertext encoding")
    }
    val cipher = Cipher.getInstance(TRANSFORMATION)
    cipher.init(Cipher.DECRYPT_MODE, key, GCMParameterSpec(GCM_TAG_LENGTH_BITS, iv))
    // AEADBadTagException propagates from doFinal on any auth-tag
    // failure (corrupted/truncated ciphertext, or a key that never
    // matched this envelope) -- caught by the command handler, never
    // caught here, so it is never mistaken for a successful decrypt.
    val plaintext = cipher.doFinal(ciphertext)
    return String(plaintext, Charsets.UTF_8)
  }

  @Command
  fun putCredential(invoke: Invoke) {
    val args = try {
      invoke.parseArgs(PutCredentialArgs::class.java)
    } catch (ex: Exception) {
      resolveError(invoke, "provider_failure", ex)
      return
    }
    try {
      val envelope = encrypt(args.secret)
      // `SharedPreferences.Editor.commit()` performs a single synchronous,
      // atomic replace of the backing XML file (write-to-temp-then-rename
      // internally) -- a concurrent reader never observes a partially
      // written envelope, and a failed encryption above never reaches
      // this line at all, leaving any prior valid record untouched.
      val committed = prefs().edit().putString(args.storageKey, envelope).commit()
      if (!committed) {
        resolveTypedError(invoke, "io_error")
        return
      }
      val response = JSObject()
      response.put("status", "ok")
      invoke.resolve(response)
    } catch (ex: KeyPermanentlyInvalidatedException) {
      resolveError(invoke, "key_unavailable", ex)
    } catch (ex: UnrecoverableKeyException) {
      resolveError(invoke, "key_unavailable", ex)
    } catch (ex: GeneralSecurityException) {
      resolveError(invoke, "provider_failure", ex)
    } catch (ex: IOException) {
      resolveError(invoke, "io_error", ex)
    } catch (ex: Exception) {
      resolveError(invoke, "provider_failure", ex)
    }
  }

  @Command
  fun getCredential(invoke: Invoke) {
    val args = try {
      invoke.parseArgs(GetCredentialArgs::class.java)
    } catch (ex: Exception) {
      resolveError(invoke, "provider_failure", ex)
      return
    }
    try {
      val envelope = prefs().getString(args.storageKey, null)
      if (envelope == null) {
        val response = JSObject()
        response.put("status", "not_found")
        invoke.resolve(response)
        return
      }
      val key = getExistingSecretKey()
      if (key == null) {
        resolveTypedError(invoke, "key_unavailable")
        return
      }
      val secret = try {
        decrypt(envelope, key)
      } catch (ex: EnvelopeFormatException) {
        resolveError(invoke, "corrupt_envelope", ex)
        return
      } catch (ex: AEADBadTagException) {
        resolveError(invoke, "auth_failed", ex)
        return
      }
      val response = JSObject()
      response.put("status", "found")
      response.put("secret", secret)
      invoke.resolve(response)
    } catch (ex: KeyPermanentlyInvalidatedException) {
      resolveError(invoke, "key_unavailable", ex)
    } catch (ex: GeneralSecurityException) {
      resolveError(invoke, "provider_failure", ex)
    } catch (ex: Exception) {
      resolveError(invoke, "provider_failure", ex)
    }
  }

  /** WI067 Checkpoint E: honest, measured Keystore proof -- `dumpsys
   * keystore2` is not available as a shell-inspectable service on this
   * build, so this command reports what the app's own API can observe
   * instead: whether the alias currently exists, and, only if a key
   * exists, its actually-measured `KeyInfo.isInsideSecureHardware()`.
   * Never claims StrongBox/hardware backing beyond what is measured here,
   * and exposes no way to export or read the key's raw bytes -- there is
   * no such operation available for an AndroidKeyStore-backed key in the
   * first place. */
  @Command
  fun keyInfo(invoke: Invoke) {
    try {
      val key = getExistingSecretKey()
      val response = JSObject()
      if (key == null) {
        response.put("exists", false)
        invoke.resolve(response)
        return
      }
      response.put("exists", true)
      try {
        val factory = javax.crypto.SecretKeyFactory.getInstance(key.algorithm, ANDROID_KEYSTORE)
        val info = factory.getKeySpec(key, android.security.keystore.KeyInfo::class.java)
          as android.security.keystore.KeyInfo
        response.put("insideSecureHardware", info.isInsideSecureHardware)
      } catch (ex: Exception) {
        response.put("insideSecureHardware", null)
      }
      invoke.resolve(response)
    } catch (ex: Exception) {
      resolveError(invoke, "provider_failure", ex)
    }
  }

  @Command
  fun deleteCredential(invoke: Invoke) {
    val args = try {
      invoke.parseArgs(DeleteCredentialArgs::class.java)
    } catch (ex: Exception) {
      resolveError(invoke, "provider_failure", ex)
      return
    }
    try {
      // Removes only this one record -- never `Editor.clear()`, never
      // touches the Keystore key, never affects any other connection's
      // entry in this same preferences file.
      val committed = prefs().edit().remove(args.storageKey).commit()
      if (!committed) {
        resolveTypedError(invoke, "io_error")
        return
      }
      val response = JSObject()
      response.put("status", "ok")
      invoke.resolve(response)
    } catch (ex: Exception) {
      resolveError(invoke, "provider_failure", ex)
    }
  }

  private fun resolveTypedError(invoke: Invoke, reason: String) {
    val response = JSObject()
    response.put("status", "error")
    response.put("reason", reason)
    invoke.resolve(response)
  }

  /** Mirrors `SafAcquisitionPlugin.resolveError`'s own discipline, one
   * step stricter: `ex.message` is never logged at all here, not even
   * redacted, since a `Cipher`/`KeyStore` exception message is not a
   * value this plugin can prove never embeds sensitive material the way
   * `SafAcquisitionPlugin`'s narrower `content://` redaction can. Only the
   * fixed reason code and the exception's class name are logged. */
  private fun resolveError(invoke: Invoke, reason: String, ex: Exception) {
    Logger.error("RepopactCredentialPlugin", "$reason (${ex.javaClass.simpleName})", null)
    resolveTypedError(invoke, reason)
  }
}
