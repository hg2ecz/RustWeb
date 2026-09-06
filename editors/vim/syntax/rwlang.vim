" Vim syntax file for RWLang
if exists("b:current_syntax") | finish | endif

syn keyword rwlangDecl mod model enum object form route
syn keyword rwlangFn fn page action query component layout
syn keyword rwlangFlow let set while if return with resource transaction authorize canonical
syn keyword rwlangRoute GET POST PUT PATCH DELETE query json upload validate auth rate cache invalidate owner role
syn keyword rwlangBuiltin html sql redirect json slugify sin cos sqrt abs monotonicNanos toF32 arrayF32 len stringLen trim lower upper contains startsWith endsWith replace split dict containsKey removeKey regexMatch regexReplace regexCaptures true false
syn keyword rwlangType String Email Url Slug Int F32 Bool Date DateTime Uuid Decimal Image Upload Db Transaction Html Json Redirect PageContext ActionContext PageError DbError Result List Void
syn keyword rwlangValidation length range pattern same
syn match rwlangTodo /\<\(TODO\|FIXME\|XXX\)\>/ contained
syn region rwlangComment start="//" end="$" contains=rwlangTodo
syn region rwlangString start=/"/ skip=/\\"/ end=/"/
syn match rwlangNumber /\<-\=\d\+\(\.\d\+f32\)\?\>/
syn match rwlangTemplate /{{\s*[^}][^}]*\s*}}/
syn match rwlangRouteHelper /@\(action\|href\)([^)]*)/
syn match rwlangOperator /=>\|->\|==\|!=\|<=\|>=\|[+*\/=<>?-]/

hi def link rwlangDecl Statement
hi def link rwlangFn Keyword
hi def link rwlangFlow Keyword
hi def link rwlangRoute Special
hi def link rwlangBuiltin Function
hi def link rwlangType Type
hi def link rwlangValidation Special
hi def link rwlangComment Comment
hi def link rwlangTodo Todo
hi def link rwlangString String
hi def link rwlangNumber Number
hi def link rwlangTemplate Identifier
hi def link rwlangRouteHelper Function
hi def link rwlangOperator Operator

let b:current_syntax = "rwlang"
