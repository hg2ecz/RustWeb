# 14. Komponensek és layoutok

Az M29 célja a közös HTML-részek újrafelhasználása raw HTML escape hatch nélkül.

## Component

```text
component fn ArticleCard(article: Article) -> Html {
    html {
        <article class="article-card">
            <h2>{{ article.title }}</h2>
            <a @href(article, article.id)>Read</a>
        </article>
    }
}
```

Használat:

```text
@component(ArticleCard, article)
```

A paraméterek compile-time typedak. Támogatott:

```text
String, Int, Bool, Date, DateTime, Uuid, Decimal
Model
Model?
List<Model>
```

`Upload` template paraméterként nem támogatott.

## Layout

```text
layout fn Main(title: String) -> Html {
    html {
        <html>
            <head><title>{{ title }}</title></head>
            <body>
                <header>Example News</header>
                <main>@content</main>
            </body>
        </html>
    }
}
```

Használat page-ben:

```text
return Ok(html {
    @layout(Main, "Article") {
        @component(ArticleCard, article)
    }
})
```

A layout pontosan egy `@content` slotot tartalmazhat.

## Miért biztonságos?

A component/layout:

- csak explicit typed paramétereket lát;
- nem kap automatikusan `ctx`, `db`, session, CSRF vagy auth capability-t;
- minden `{{ ... }}` továbbra is HTML-escaped;
- `@href/@action` továbbra is typed route helper;
- `@component`, `@layout`, `@content` csak HTML content pozícióban használható, attribútumban nem;
- compiler tiltja a component/layout ciklusokat.

Nincs raw HTML visszatérési érték és nincs `unsafeHtml(...)` escape hatch.

## Modell és lista átadása

```text
component fn ArticleList(items: List<Article>) -> Html {
    html {
        <section>
            @for article in items {
                @component(ArticleCard, article)
            }
        </section>
    }
}
```

Optional model:

```text
component fn MaybeArticle(article: Article?) -> Html {
    html {
        @if article {
            @component(ArticleCard, article)
        }
    }
}
```

## Cache

A component/layout nem változtatja meg a public-cache szabályokat. Ha egy cached page request-specifikus értéket ad át komponensnek vagy layoutnak, a compiler ugyanúgy elutasítja a public cache-t.

## Tudatos M29 korlát

A component és layout tisztán renderelő template: nincs benne `let`, query, transaction vagy side effect. Az adatot a page készíti elő, a template csak megjeleníti.
