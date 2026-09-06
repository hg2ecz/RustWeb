# 9. Upload és AppFs

## Route

```text
route uploadFile POST "/upload"
    upload file<Upload> to "uploads"
    => uploadFile;
```

## Action

```text
action fn uploadFile(ctx: ActionContext, file: Upload)
    -> Result<Redirect, PageError> {
    let savedPath = file.path;
    let originalName = file.filename;
    let contentType = file.contentType;
    let byteCount = file.bytes;
    return Ok(redirect("/uploaded"));
}
```

Az `Upload` csak metadata; a teljes fájl nem kerül a VM heapbe.

## Server config

```bash
--data-root /srv/app/data
--fs-mode rwc
--max-upload-bytes 16777216
```

Uploadhoz `c` és `w` szükséges.

## Security

A kliens filename/MIME csak metadata. A runtime random storage key-t használ, staging fájlba streamel, majd atomikusan commitol. Linuxon az AppFs `openat2` confinementet használ (`BENEATH`, no symlink/magic-link/xdev).

Action/DB hiba esetén a runtime cleanupot kísérel meg.

## Typed image upload (M33)

For CMS/media use, prefer `Image` over generic `Upload`:

```rwlang
route uploadHero POST "/admin/hero" upload hero<Image> to "media" auth user => uploadHero;
```

The server validates PNG/JPEG bytes after streaming them into the confined AppFs location. Client MIME and filename are not trusted. Image routes require `rwc` AppFs mode. See [Biztonságos képek és media library](17-media-library.md).
