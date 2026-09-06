# Canonical URL és régi slugok

CMS-nél és hírportálnál gyakori, hogy egy cikk publikus slugja megváltozik, de a régi linkeknek továbbra is működniük kell. RWLangban ezt ne kézzel összerakott redirect URL-lel kezeld.

## Ajánlott minta

Az adatbázisban az objektum mindig a jelenlegi canonical slugot tárolja. A régi slugokat külön alias/history tábla tartja nyilván.

```rwlang
object Article {
    model {
        id: Int
        slug: Slug
        title: String
    }

    query fn resolveBySlug(db: Db, slug: Slug) -> Result<Article, DbError> sql {
        SELECT a.id AS id, a.slug AS slug, a.title AS title
        FROM articles a
        LEFT JOIN article_slug_aliases old ON old.article_id = a.id
        WHERE a.slug = :slug OR old.slug = :slug
    }

    page fn show(ctx: PageContext, db: Db, slug: Slug) -> Result<Html, PageError> {
        let article = Article.resolveBySlug(db, slug)?;
        canonical slug slug from article.slug;
        return Ok(html {<h1>{{ article.title }}</h1>});
    }
}

route article GET "/cikk/:slug<Slug>" => Article.show;
```

Ha a kérés `/cikk/regi-cim`, de a rekord `article.slug` értéke `uj-cim`, a runtime ezt adja:

```text
301 Moved Permanently
Location: /cikk/uj-cim
```

Ha a kért slug már canonical, nincs redirect.

## Miért jobb ez egy `redirect(url)` hívásnál?

A canonical URL-t a runtime ugyanabból a compiler-owned route-ból építi újra. A `.rw` kód nem adhat meg hostot vagy tetszőleges redirect URL-t, ezért ez a feature nem nyit open-redirect felületet.

A paraméterek typed értékekből kerülnek vissza az URL-be. Csak a route-ban deklarált query mezők maradnak meg; ismeretlen raw query paramétert a runtime nem tükröz át a `Location` headerbe.

## Szabályok

- a bal oldali név valódi `Slug` path-paraméter legyen;
- a canonical expression szintén `Slug` legyen;
- page-enként egy canonical slug invariant támogatott;
- top-level statement legyen;
- public page cache-sel nem kombinálható;
- a slugot title-változáskor ne írd át automatikusan, ha az üzleti/SEO policy nem ezt kívánja;
- régi alias legyen DB-ben unique.

## 301 és 303 nem ugyanaz

A canonical GET átirányítás `301 Moved Permanently`. A POST actionök eddigi redirectje továbbra is `303 See Other`. A POST/Redirect/GET és flash-message UX külön milestone-ban készül el.
