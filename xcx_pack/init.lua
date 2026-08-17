-- Zap Nano Plugin: xcx_pack
return {
    name = "xcx_pack",
    author = "Zap VIM",
    version = "1.0.0",
    on_load = function()
        zap.register_highlighter("xcx", {
            -- Multi-line Comments
            { pattern = "---(.|\n)*?\\*---", color = "DarkGray" },
            -- Single-line Comments
            { pattern = "---[^\n]*", color = "DarkGray" },
            -- Controls & Types (Keywords)
            { pattern = "\\b(if|then|else|end|func|fiber|const|while|do|for|in|to|yield|from|return|as|include)\\b", color = "Keyword" },
            -- Types and Identifiers
            { pattern = "\\b(database|table|map|json|set|array|serve|i|f|s|b):", color = "Special" },
            -- Standard Library Modules
            { pattern = "\\b(net|crypto|store|env|random|date|perf|json|halt)\\.", color = "Operator" },
            { pattern = "\\b(halt\\.alert|halt\\.error|halt\\.fatal)\\b", color = "LightRed" },
            -- Standard Library Methods
            { pattern = "\\.\\b(parse|get|bind|set|push|size|count|keys|toStr|inject|first|post|put|delete|request|has|close|isOpen|respond|sync|drop|fetch|where|queryRaw|query|save|insert|exec|truncate|remove|begin|commit|rollback|hash|verify|token|write|read|append|exists|list|isDir|mkdir|glob|zip|unzip|args|choice|int|float|now|ms|us|ns|next|run|isDone|lower|upper)\\b", color = "Function" },
            -- IO Directive
            { pattern = ">!", color = "LightRed" },
            -- Booleans and Null
            { pattern = "\\b(true|false|null|EMPTY)\\b", color = "String" },
            -- Attributes
            { pattern = "(@auto|@pk|@unique|@optional|@default|@fk|@step)", color = "Special" },
            -- Tables definition blocks
            { pattern = "\\b(columns|rows)\\b", color = "Keyword" },
            -- Numbers
            { pattern = "\\b\\d+(\\.\\d+)?\\b", color = "LightRed" },
            -- Object / map / table / schema connectors
            { pattern = "::", color = "Operator" },
            { pattern = "<\\-<", color = "Operator" },
            { pattern = "<<<", color = "Operator" },
            { pattern = ">>>", color = "Operator" },
            -- Strings
            { pattern = "\"[^\"]*\"", color = "String" },
        })
        zap.register_snippets("xcx", {
            { prefix = "for", body = "for item in collection do;\n    |\nend;", description = "For loop" },
            { prefix = "func", body = "func name(i: arg -> i) {\n    |\n};", description = "Function definition" },
            { prefix = "fiber", body = "fiber name(json: req -> json) {\n    |\n};", description = "Fiber handler definition" },
            { prefix = "database", body = "database: app {\n    engine = \"sqlite\",\n    path   = \"data.db\"\n};", description = "Database creation" },
            { prefix = "table", body = "table: users {\n    columns = [\n        |\n    ]\n    rows = [EMPTY]\n};", description = "Table schema" },
            { prefix = "serve", body = "serve: api {\n    port    = 8080,\n    host    = \"127.0.0.1\",\n    workers = 4,\n    routes  = [\n        |\n    ]\n};", description = "HTTP router" }
        })
        zap.print("Plugin xcx_pack loaded fully!")
    end
}
