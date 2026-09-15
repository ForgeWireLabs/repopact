// Decision 0056/0057, WI065 Checkpoint B: the narrow Android SAF bridge.
//
// This class owns only what the Android platform forces it to own
// (ActivityResult handling, DocumentsContract/DocumentFile traversal,
// ContentResolver streaming, URI-permission lifecycle). It performs no
// path-safety validation, no collision detection, no resource-bound
// enforcement, and no ZIP parsing -- all of that remains exclusively
// Checkpoint A's (`repopact-mobile-acquisition`) responsibility on the Rust
// side. Every response this class returns is a small, fixed-shape JSON
// object; a display name flowing back to Rust is treated as fully
// untrusted there (WI065 Checkpoint B §10).
//
// Structured after the exact `@TauriPlugin`/`Plugin(activity)`/`@Command`/
// `startActivityForResult`/`@ActivityCallback` pattern this repository's
// own vendored `tauri-plugin-dialog` 2.7.3 `DialogPlugin.kt` uses against
// this same Tauri 2.11.5 mobile-plugin runtime -- not a generic example.

package com.forgewirelabs.repopact.mobileacquisition

import android.app.Activity
import android.content.Intent
import android.database.Cursor
import android.net.Uri
import android.provider.DocumentsContract
import androidx.activity.result.ActivityResult
import app.tauri.Logger
import app.tauri.annotation.ActivityCallback
import app.tauri.annotation.Command
import app.tauri.annotation.InvokeArg
import app.tauri.annotation.TauriPlugin
import app.tauri.plugin.Invoke
import app.tauri.plugin.JSArray
import app.tauri.plugin.JSObject
import app.tauri.plugin.Plugin
import java.io.File
import java.io.IOException
import java.util.UUID

/** Bounded safety net for one document copy -- see `openDocument`'s doc
 * comment. This is *not* the resource-accounting authority; Checkpoint A's
 * bounded importer always re-counts actual bytes on the Rust side
 * regardless of what this cap allowed through. */
private const val MAX_STAGED_DOCUMENT_BYTES: Long = 4L * 1024 * 1024 * 1024 // 4 GiB

private const val ZIP_MIME_TYPE = "application/zip"

@InvokeArg
class OpenDocumentArgs {
  lateinit var uri: String
}

@InvokeArg
class ListChildrenArgs {
  lateinit var treeUri: String
  lateinit var parentUri: String
}

@TauriPlugin
class SafAcquisitionPlugin(private val activity: Activity) : Plugin(activity) {

  @Command
  fun pickDirectoryTree(invoke: Invoke) {
    try {
      val intent = Intent(Intent.ACTION_OPEN_DOCUMENT_TREE)
      startActivityForResult(invoke, intent, "directoryPickResult")
    } catch (ex: Exception) {
      resolveError(invoke, "activity_unavailable", ex)
    }
  }

  @ActivityCallback
  fun directoryPickResult(invoke: Invoke, result: ActivityResult) {
    when (result.resultCode) {
      Activity.RESULT_CANCELED -> {
        val response = JSObject()
        response.put("status", "cancelled")
        invoke.resolve(response)
      }
      Activity.RESULT_OK -> {
        val uri = result.data?.data
        if (uri == null) {
          resolveTypedError(invoke, "missing_uri")
          return
        }
        // Decision 0057 §"Persistable URI permissions": a directory-tree
        // grant is retained (only for this tree, read-only) because a
        // future export/divergence-check operation may need to re-open it
        // without prompting the user again. Providers that refuse
        // persistable grants are handled honestly -- the import still
        // proceeds using the one-shot grant the picker Intent itself
        // already carries, and the workspace is simply not eligible for a
        // permission-free future re-open; this is never treated as a
        // reason to request broader storage authority.
        try {
          activity.contentResolver.takePersistableUriPermission(
            uri,
            Intent.FLAG_GRANT_READ_URI_PERMISSION
          )
        } catch (ex: SecurityException) {
          Logger.info("SafAcquisitionPlugin", "provider does not support persistable grants: ${ex.message}")
        }

        val displayName = try {
          DocumentsContract.getTreeDocumentId(uri)?.substringAfterLast('/') ?: "workspace"
        } catch (ex: Exception) {
          "workspace"
        }

        val response = JSObject()
        response.put("status", "selected")
        response.put("treeUri", uri.toString())
        response.put("displayName", displayName)
        invoke.resolve(response)
      }
      else -> resolveTypedError(invoke, "provider_failure")
    }
  }

  @Command
  fun pickArchiveDocument(invoke: Invoke) {
    try {
      val intent = Intent(Intent.ACTION_OPEN_DOCUMENT)
      intent.addCategory(Intent.CATEGORY_OPENABLE)
      intent.type = ZIP_MIME_TYPE
      // MIME filtering here is a UX convenience only (WI065 Checkpoint B
      // §13) -- many providers report `application/octet-stream` for
      // `.zip` files regardless, so the extra MIME type keeps the picker
      // usable without being the actual format authority. The Rust ZIP
      // importer (Checkpoint A) remains the real validator: an invalid
      // selection surfaces as `ArchiveInvalid` there, never assumed valid
      // here merely because it passed this filter.
      intent.putExtra(
        Intent.EXTRA_MIME_TYPES,
        arrayOf(ZIP_MIME_TYPE, "application/octet-stream", "application/x-zip-compressed")
      )
      startActivityForResult(invoke, intent, "archivePickResult")
    } catch (ex: Exception) {
      resolveError(invoke, "activity_unavailable", ex)
    }
  }

  @ActivityCallback
  fun archivePickResult(invoke: Invoke, result: ActivityResult) {
    when (result.resultCode) {
      Activity.RESULT_CANCELED -> {
        val response = JSObject()
        response.put("status", "cancelled")
        invoke.resolve(response)
      }
      Activity.RESULT_OK -> {
        val uri = result.data?.data
        if (uri == null) {
          resolveTypedError(invoke, "missing_uri")
          return
        }
        // Decision 0057: archive picks are one-shot input. No persistable
        // grant is requested -- the one-shot grant the picker Intent
        // itself carries is sufficient for the immediate `openDocument`
        // call this operation makes next.
        val displayName = queryDisplayName(uri) ?: uri.lastPathSegment ?: "archive.zip"
        val response = JSObject()
        response.put("status", "selected")
        response.put("documentUri", uri.toString())
        response.put("displayName", displayName)
        invoke.resolve(response)
      }
      else -> resolveTypedError(invoke, "provider_failure")
    }
  }

  /**
   * Lists the immediate children of one SAF directory node. Called once
   * per directory the Rust-side [AcquisitionSource] actually visits
   * (WI065 Checkpoint B §9) -- never the whole tree at once. `parentUri`
   * is `treeUri` itself for the root call.
   */
  @Command
  fun listChildren(invoke: Invoke) {
    val args = try {
      invoke.parseArgs(ListChildrenArgs::class.java)
    } catch (ex: Exception) {
      resolveError(invoke, "provider_failure", ex)
      return
    }
    try {
      val treeUri = Uri.parse(args.treeUri)
      val parentUri = Uri.parse(args.parentUri)
      val parentDocumentId = DocumentsContract.getDocumentId(parentUri)
      val childrenUri =
        DocumentsContract.buildChildDocumentsUriUsingTree(treeUri, parentDocumentId)

      val entries = JSArray()
      val projection = arrayOf(
        DocumentsContract.Document.COLUMN_DOCUMENT_ID,
        DocumentsContract.Document.COLUMN_DISPLAY_NAME,
        DocumentsContract.Document.COLUMN_MIME_TYPE,
        DocumentsContract.Document.COLUMN_SIZE
      )
      val cursor: Cursor? = activity.contentResolver.query(childrenUri, projection, null, null, null)
      cursor.use { c ->
        if (c == null) {
          resolveTypedError(invoke, "provider_failure")
          return
        }
        val idIndex = c.getColumnIndex(DocumentsContract.Document.COLUMN_DOCUMENT_ID)
        val nameIndex = c.getColumnIndex(DocumentsContract.Document.COLUMN_DISPLAY_NAME)
        val mimeIndex = c.getColumnIndex(DocumentsContract.Document.COLUMN_MIME_TYPE)
        val sizeIndex = c.getColumnIndex(DocumentsContract.Document.COLUMN_SIZE)
        while (c.moveToNext()) {
          val documentId = if (idIndex >= 0) c.getString(idIndex) else null
          if (documentId == null) {
            // §11: a child with no stable document id cannot be addressed
            // by any later listChildren/openDocument call -- surfaced as a
            // typed failure rather than silently skipped (a silent skip
            // would make the imported tree quietly incomplete).
            resolveTypedError(invoke, "provider_failure")
            return
          }
          val childUri = DocumentsContract.buildDocumentUriUsingTree(treeUri, documentId)
          val mimeType = if (mimeIndex >= 0) c.getString(mimeIndex) else null
          val isDirectory = mimeType == DocumentsContract.Document.MIME_TYPE_DIR
          val entry = JSObject()
          entry.put("uri", childUri.toString())
          // A null/blank display name is passed through as JSON null
          // rather than defaulted here -- Rust's AndroidSafSource treats a
          // missing name as a typed `unsupported_entry` failure (WI065
          // Checkpoint B §10/§11) rather than this layer inventing a name
          // that could collide with another real entry.
          val displayName = if (nameIndex >= 0) c.getString(nameIndex) else null
          entry.put("displayName", displayName)
          entry.put("isDirectory", isDirectory)
          val size = if (sizeIndex >= 0 && !c.isNull(sizeIndex)) c.getLong(sizeIndex) else null
          entry.put("size", size)
          entries.put(entry)
        }
      }

      val response = JSObject()
      response.put("status", "ok")
      response.put("entries", entries)
      invoke.resolve(response)
    } catch (ex: SecurityException) {
      resolveError(invoke, "permission_denied", ex)
    } catch (ex: IOException) {
      resolveError(invoke, "io_error", ex)
    } catch (ex: Exception) {
      resolveError(invoke, "provider_failure", ex)
    }
  }

  /**
   * Copies one SAF document's bytes into an app-private staging file
   * (`cacheDir/saf-stage/<uuid>`) and returns that file's path. See
   * `repopact-mobile-saf`'s `mobile.rs` (`open_document_to_staging`'s doc
   * comment) for why a staging file, rather than a stream or an fd, is
   * what actually crosses the Rust/Kotlin boundary in this Tauri version.
   *
   * [MAX_STAGED_DOCUMENT_BYTES] is a safety net against one pathological
   * single-document copy consuming unbounded app-private storage before
   * Rust ever gets a chance to apply Checkpoint A's own (authoritative)
   * per-file and total-bytes bounds -- it is not itself the resource
   * accounting authority.
   */
  @Command
  fun openDocument(invoke: Invoke) {
    val args = try {
      invoke.parseArgs(OpenDocumentArgs::class.java)
    } catch (ex: Exception) {
      resolveError(invoke, "provider_failure", ex)
      return
    }
    val uri = Uri.parse(args.uri)
    val stagingDir = File(activity.cacheDir, "saf-stage")
    if (!stagingDir.exists() && !stagingDir.mkdirs()) {
      resolveTypedError(invoke, "io_error")
      return
    }
    val stagingFile = File(stagingDir, "${UUID.randomUUID()}.bin")
    try {
      activity.contentResolver.openInputStream(uri).use { input ->
        if (input == null) {
          resolveTypedError(invoke, "not_found")
          return
        }
        stagingFile.outputStream().use { output ->
          val buffer = ByteArray(256 * 1024)
          var total = 0L
          while (true) {
            val read = input.read(buffer)
            if (read < 0) break
            total += read
            if (total > MAX_STAGED_DOCUMENT_BYTES) {
              throw IOException("document exceeds the staging safety-net byte cap")
            }
            output.write(buffer, 0, read)
          }
          val response = JSObject()
          response.put("status", "ok")
          response.put("stagingPath", stagingFile.absolutePath)
          response.put("byteCount", total)
          invoke.resolve(response)
        }
      }
    } catch (ex: SecurityException) {
      stagingFile.delete()
      resolveError(invoke, "permission_denied", ex)
    } catch (ex: IOException) {
      stagingFile.delete()
      resolveError(invoke, "io_error", ex)
    } catch (ex: Exception) {
      stagingFile.delete()
      resolveError(invoke, "provider_failure", ex)
    }
  }

  private fun queryDisplayName(uri: Uri): String? {
    return try {
      activity.contentResolver.query(
        uri,
        arrayOf(DocumentsContract.Document.COLUMN_DISPLAY_NAME),
        null,
        null,
        null
      )?.use { cursor ->
        if (cursor.moveToFirst()) {
          val index = cursor.getColumnIndex(DocumentsContract.Document.COLUMN_DISPLAY_NAME)
          if (index >= 0) cursor.getString(index) else null
        } else {
          null
        }
      }
    } catch (ex: Exception) {
      null
    }
  }

  private fun resolveTypedError(invoke: Invoke, reason: String) {
    val response = JSObject()
    response.put("status", "error")
    response.put("reason", reason)
    invoke.resolve(response)
  }

  private fun resolveError(invoke: Invoke, reason: String, ex: Exception) {
    Logger.error("SafAcquisitionPlugin", "$reason: ${ex.message}", ex)
    resolveTypedError(invoke, reason)
  }
}
