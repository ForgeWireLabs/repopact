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

// WI065 Checkpoint D: explicit SAF export/share-back argument shapes.

@InvokeArg
class CreateExportRootArgs {
  lateinit var treeUri: String
  lateinit var name: String
}

@InvokeArg
class CreateChildDocumentArgs {
  lateinit var parentUri: String
  lateinit var name: String
  var isDirectory: Boolean = false
}

@InvokeArg
class WriteDocumentArgs {
  lateinit var uri: String
  lateinit var stagingPath: String
}

@InvokeArg
class DeleteDocumentArgs {
  lateinit var uri: String
}

@InvokeArg
class CreateExportArchiveArgs {
  lateinit var suggestedName: String
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
      // WI065 Checkpoint B.5: for the root call, Rust's AndroidSafSource
      // passes the tree URI itself as parentUri (there is no document URI
      // for the tree's root yet) -- DocumentsContract.getDocumentId()
      // throws IllegalArgumentException on a tree URI, it only accepts a
      // document URI built via buildDocumentUriUsingTree. Every subsequent
      // call passes a real document URI (the one this method itself builds
      // below), where isDocumentUri() is true. This branch is required for
      // the very first listChildren call on any picked tree to succeed.
      val parentDocumentId = if (DocumentsContract.isDocumentUri(activity, parentUri)) {
        DocumentsContract.getDocumentId(parentUri)
      } else {
        DocumentsContract.getTreeDocumentId(parentUri)
      }
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

  // -------------------------------------------------------------------
  // WI065 Checkpoint D: explicit SAF export/share-back.
  //
  // Every method below owns only DocumentsContract/ContentResolver
  // mechanics -- name collision comparison, resource bounds, cancellation,
  // and traversal order all remain exclusively
  // `repopact_mobile_acquisition::export`'s responsibility on the Rust
  // side, exactly mirroring the import methods above.
  // -------------------------------------------------------------------

  /** Picks a SAF destination *parent* tree for a directory export. Reuses
   * the same [directoryPickResult] callback and response shape as
   * [pickDirectoryTree] -- "pick a directory tree" is the same operation
   * either way; only the caller-side intent differs. */
  @Command
  fun pickExportDirectory(invoke: Invoke) {
    try {
      val intent = Intent(Intent.ACTION_OPEN_DOCUMENT_TREE)
      startActivityForResult(invoke, intent, "directoryPickResult")
    } catch (ex: Exception) {
      resolveError(invoke, "activity_unavailable", ex)
    }
  }

  /**
   * Creates exactly one new export root beneath the picked parent tree
   * (WI065 Checkpoint D §6/§7). Checks for a same-named existing child
   * first under Decision 0057's case-insensitive comparison, and reports a
   * typed conflict rather than ever silently merging into or overwriting
   * an existing item.
   */
  @Command
  fun createExportRoot(invoke: Invoke) {
    val args = try {
      invoke.parseArgs(CreateExportRootArgs::class.java)
    } catch (ex: Exception) {
      resolveError(invoke, "provider_failure", ex)
      return
    }
    try {
      val treeUri = Uri.parse(args.treeUri)
      val parentDocumentId = DocumentsContract.getTreeDocumentId(treeUri)
      val parentUri = DocumentsContract.buildDocumentUriUsingTree(treeUri, parentDocumentId)
      val childrenUri = DocumentsContract.buildChildDocumentsUriUsingTree(treeUri, parentDocumentId)

      val existing = activity.contentResolver.query(
        childrenUri,
        arrayOf(DocumentsContract.Document.COLUMN_DISPLAY_NAME),
        null,
        null,
        null
      )?.use { cursor ->
        val nameIndex = cursor.getColumnIndex(DocumentsContract.Document.COLUMN_DISPLAY_NAME)
        var found = false
        while (cursor.moveToNext()) {
          val childName = if (nameIndex >= 0) cursor.getString(nameIndex) else null
          if (childName != null && childName.equals(args.name, ignoreCase = true)) {
            found = true
            break
          }
        }
        found
      } ?: false

      if (existing) {
        val response = JSObject()
        response.put("status", "conflict")
        invoke.resolve(response)
        return
      }

      val createdUri = DocumentsContract.createDocument(
        activity.contentResolver,
        parentUri,
        DocumentsContract.Document.MIME_TYPE_DIR,
        args.name
      )
      if (createdUri == null) {
        resolveTypedError(invoke, "provider_failure")
        return
      }
      val response = JSObject()
      response.put("status", "ok")
      response.put("rootUri", createdUri.toString())
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
   * Creates one child document (directory or file) beneath an
   * already-known parent document URI. Called once per entry the Rust-side
   * exporter writes -- never a bulk/recursive create.
   */
  @Command
  fun createChildDocument(invoke: Invoke) {
    val args = try {
      invoke.parseArgs(CreateChildDocumentArgs::class.java)
    } catch (ex: Exception) {
      resolveError(invoke, "provider_failure", ex)
      return
    }
    try {
      val parentUri = Uri.parse(args.parentUri)
      val mimeType = if (args.isDirectory) {
        DocumentsContract.Document.MIME_TYPE_DIR
      } else {
        // A generic, permissive type -- the exported bytes are exact
        // regardless of what MIME a provider records, and `args.name`
        // already carries the correct extension (WI065 Checkpoint D §11).
        "application/octet-stream"
      }
      val createdUri = DocumentsContract.createDocument(
        activity.contentResolver,
        parentUri,
        mimeType,
        args.name
      )
      if (createdUri == null) {
        resolveTypedError(invoke, "provider_failure")
        return
      }
      val response = JSObject()
      response.put("status", "ok")
      response.put("uri", createdUri.toString())
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
   * Copies a completed local staging file's bytes into an already-created
   * SAF destination document (WI065 Checkpoint D: the outbound mirror of
   * `openDocument`'s staging-file pattern). Used once per exported file
   * during a directory export, and once for a completed archive during
   * archive export.
   */
  @Command
  fun writeDocumentFromStagingFile(invoke: Invoke) {
    val args = try {
      invoke.parseArgs(WriteDocumentArgs::class.java)
    } catch (ex: Exception) {
      resolveError(invoke, "provider_failure", ex)
      return
    }
    val stagingFile = File(args.stagingPath)
    if (!stagingFile.isFile) {
      resolveTypedError(invoke, "not_found")
      return
    }
    try {
      activity.contentResolver.openOutputStream(Uri.parse(args.uri)).use { output ->
        if (output == null) {
          resolveTypedError(invoke, "not_found")
          return
        }
        stagingFile.inputStream().use { input ->
          val buffer = ByteArray(256 * 1024)
          var total = 0L
          while (true) {
            val read = input.read(buffer)
            if (read < 0) break
            output.write(buffer, 0, read)
            total += read
          }
          val response = JSObject()
          response.put("status", "ok")
          response.put("byteCount", total)
          invoke.resolve(response)
        }
      }
    } catch (ex: SecurityException) {
      resolveError(invoke, "permission_denied", ex)
    } catch (ex: IOException) {
      resolveError(invoke, "io_error", ex)
    } catch (ex: Exception) {
      resolveError(invoke, "provider_failure", ex)
    }
  }

  /**
   * Deletes a document -- used only for best-effort cleanup of an
   * app-created export root after a failed or cancelled directory export
   * (Decision 0057 §17). Never called on anything the user selected
   * themselves.
   */
  @Command
  fun deleteDocument(invoke: Invoke) {
    val args = try {
      invoke.parseArgs(DeleteDocumentArgs::class.java)
    } catch (ex: Exception) {
      resolveError(invoke, "provider_failure", ex)
      return
    }
    try {
      val deleted = DocumentsContract.deleteDocument(activity.contentResolver, Uri.parse(args.uri))
      if (!deleted) {
        resolveTypedError(invoke, "provider_failure")
        return
      }
      val response = JSObject()
      response.put("status", "ok")
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
   * Picks a brand-new `CreateDocument` destination for an archive export
   * (WI065 Checkpoint D §10). Always a new document -- Stage 1 never
   * offers to overwrite an existing archive in place.
   */
  @Command
  fun createExportArchive(invoke: Invoke) {
    val args = try {
      invoke.parseArgs(CreateExportArchiveArgs::class.java)
    } catch (ex: Exception) {
      resolveError(invoke, "provider_failure", ex)
      return
    }
    try {
      val intent = Intent(Intent.ACTION_CREATE_DOCUMENT)
      intent.addCategory(Intent.CATEGORY_OPENABLE)
      intent.type = ZIP_MIME_TYPE
      intent.putExtra(Intent.EXTRA_TITLE, args.suggestedName)
      startActivityForResult(invoke, intent, "createExportArchiveResult")
    } catch (ex: Exception) {
      resolveError(invoke, "activity_unavailable", ex)
    }
  }

  @ActivityCallback
  fun createExportArchiveResult(invoke: Invoke, result: ActivityResult) {
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
        val displayName = queryDisplayName(uri) ?: uri.lastPathSegment ?: "export.zip"
        val response = JSObject()
        response.put("status", "selected")
        response.put("documentUri", uri.toString())
        response.put("displayName", displayName)
        invoke.resolve(response)
      }
      else -> resolveTypedError(invoke, "provider_failure")
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
    // WI065 Checkpoint B.5 privacy audit (§19): Android's own exception
    // messages (e.g. DocumentsContract.getDocumentId's
    // IllegalArgumentException) can embed a full content:// URI, which
    // itself reveals the picked tree's on-device path structure. The
    // throwable itself is deliberately NOT passed to Logger.error -- Log.e
    // with a Throwable prints its stack trace, which re-embeds that same
    // unredacted message regardless of what string is passed alongside it.
    // The reason code and exception class name already carry enough
    // information to diagnose a real defect without the raw URI.
    Logger.error(
      "SafAcquisitionPlugin",
      "$reason (${ex.javaClass.simpleName}): ${redactContentUris(ex.message)}",
      null
    )
    resolveTypedError(invoke, reason)
  }

  private fun redactContentUris(message: String?): String {
    if (message == null) return "(no message)"
    return message.replace(Regex("content://\\S+"), "content://<redacted>")
  }
}
