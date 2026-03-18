    class DOMParser {
        parseFromString(input, mimeType) {
            const source = String(input ?? "");
            const normalizedMimeType = String(mimeType);

            if (normalizedMimeType === "text/html") {
                return parseHtmlDocument(source, normalizedMimeType);
            }

            if (
                normalizedMimeType === "text/xml" ||
                normalizedMimeType === "application/xml" ||
                normalizedMimeType === "application/xhtml+xml" ||
                normalizedMimeType === "image/svg+xml"
            ) {
                return parseXmlDocument(source, normalizedMimeType);
            }

            throw new TypeError("Unsupported DOMParser mimeType");
        }
    }

    function parseHtmlDocument(source, mimeType) {
        const document = new Document();
        document.contentType = mimeType;
        const parsed = parseMarkup(document, source, false);

        if (parsed.error) {
            document.body.textContent = source.replace(/<[^>]*>/g, "");
            return document;
        }

        const htmlElement = parsed.nodes.find((node) => node.tagName === "HTML");
        if (htmlElement) {
            resetDocumentTree(document, htmlElement);
            document.head = htmlElement.children.find((node) => node.tagName === "HEAD") ?? new Element("head");
            document.body = htmlElement.children.find((node) => node.tagName === "BODY") ?? new Element("body");

            if (document.head.parentNode !== document.documentElement) {
                document.documentElement.appendChild(document.head);
            }
            if (document.body.parentNode !== document.documentElement) {
                document.documentElement.appendChild(document.body);
            }
            return document;
        }

        clearChildren(document.body);
        for (const node of parsed.nodes) {
            document.body.appendChild(node);
        }
        return document;
    }

    function parseXmlDocument(source, mimeType) {
        const document = new Document();
        document.contentType = mimeType;
        const parsed = parseMarkup(document, source, true);

        if (parsed.error || parsed.nodes.filter((node) => node.nodeType === 1).length !== 1) {
            return createParserErrorDocument(document, mimeType);
        }

        const root = parsed.nodes.find((node) => node.nodeType === 1);
        clearChildren(document);
        document.documentElement = root;
        document.head = null;
        document.body = null;
        document.appendChild(root);
        return document;
    }

    function createParserErrorDocument(document, mimeType) {
        const errorDocument = new Document();
        errorDocument.contentType = mimeType;
        const parserError = new Element("parsererror");
        parserError.textContent = "Invalid XML";
        clearChildren(errorDocument);
        errorDocument.documentElement = parserError;
        errorDocument.head = null;
        errorDocument.body = null;
        errorDocument.appendChild(parserError);
        return errorDocument;
    }

    function resetDocumentTree(document, documentElement) {
        clearChildren(document);
        document.documentElement = documentElement;
        document.appendChild(documentElement);
    }

    function clearChildren(node) {
        node.childNodes = [];
    }

    function parseMarkup(document, source, strictXml) {
        const stack = [];
        const nodes = [];
        const tokens = source.match(/<!--[\s\S]*?-->|<![^>]*>|<\/?[^>]+>|[^<]+/g) ?? [];

        for (const token of tokens) {
            if (!token) {
                continue;
            }

            if (token.startsWith("<!--") || token.startsWith("<!")) {
                continue;
            }

            if (token.startsWith("</")) {
                const tagName = token.slice(2, -1).trim().toUpperCase();
                const current = stack.pop();
                if (!current || current.tagName !== tagName) {
                    return { error: true, nodes: [] };
                }
                continue;
            }

            if (token.startsWith("<")) {
                const selfClosing = token.endsWith("/>");
                const content = token.slice(1, selfClosing ? -2 : -1).trim();
                const parts = content.split(/\s+/, 2);
                const tagName = parts[0];
                const element = document.createElement(tagName);
                for (const [name, value] of parseAttributes(content.slice(tagName.length))) {
                    element.setAttribute(name, value);
                }

                if (stack.length > 0) {
                    stack[stack.length - 1].appendChild(element);
                } else {
                    nodes.push(element);
                }

                if (!selfClosing) {
                    stack.push(element);
                }
                continue;
            }

            if (!strictXml && stack.length === 0 && !token.trim()) {
                continue;
            }

            const textNode = document.createTextNode(token);
            if (stack.length > 0) {
                stack[stack.length - 1].appendChild(textNode);
            } else {
                nodes.push(textNode);
            }
        }

        if (strictXml && stack.length > 0) {
            return { error: true, nodes: [] };
        }

        return { error: false, nodes };
    }

    function parseAttributes(source) {
        const attributes = [];
        const pattern = /([^\s=]+)(?:\s*=\s*(?:"([^"]*)"|'([^']*)'|([^\s"'>]+)))?/g;
        let match = pattern.exec(source);

        while (match) {
            attributes.push([match[1], match[2] ?? match[3] ?? match[4] ?? ""]);
            match = pattern.exec(source);
        }

        return attributes;
    }
