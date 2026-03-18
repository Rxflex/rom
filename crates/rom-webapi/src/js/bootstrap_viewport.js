    class ScreenOrientation extends EventTarget {
        constructor(type, angle) {
            super();
            this.type = String(type);
            this.angle = Number(angle);
            this.onchange = null;
        }

        lock() {
            return Promise.resolve();
        }

        unlock() {}
    }

    class VisualViewport extends EventTarget {
        constructor(state) {
            super();
            this.width = state.visualViewportWidth;
            this.height = state.visualViewportHeight;
            this.offsetLeft = 0;
            this.offsetTop = 0;
            this.pageLeft = 0;
            this.pageTop = 0;
            this.scale = state.devicePixelRatio;
            this.onresize = null;
            this.onscroll = null;
        }
    }

    class MediaQueryList extends EventTarget {
        constructor(query, evaluator) {
            super();
            this.media = String(query);
            this.matches = Boolean(evaluator(this.media));
            this.onchange = null;
        }

        addListener(listener) {
            this.addEventListener("change", listener);
        }

        removeListener(listener) {
            this.removeEventListener("change", listener);
        }

        dispatchEvent(event) {
            if (event?.type === "change" && typeof this.onchange === "function") {
                this.onchange(event);
            }
            return super.dispatchEvent(event);
        }
    }

    function createViewportState() {
        return {
            innerWidth: 1920,
            innerHeight: 1040,
            outerWidth: 1920,
            outerHeight: 1080,
            visualViewportWidth: 1920,
            visualViewportHeight: 1040,
            devicePixelRatio: 1,
        };
    }

    function createScreen(viewportState) {
        return {
            width: viewportState.outerWidth,
            height: viewportState.outerHeight,
            availWidth: viewportState.outerWidth,
            availHeight: viewportState.innerHeight,
            availTop: 0,
            availLeft: 0,
            colorDepth: 24,
            pixelDepth: 24,
            orientation: new ScreenOrientation("landscape-primary", 0),
        };
    }

    function createVisualViewport(viewportState) {
        return new VisualViewport(viewportState);
    }

    function createMatchMedia(viewportState) {
        return (query) => new MediaQueryList(query, (media) => evaluateMediaQuery(media, viewportState));
    }

    function evaluateMediaQuery(query, viewportState) {
        const clauses = String(query)
            .toLowerCase()
            .split(/\band\b/)
            .map((clause) => clause.replace(/[()]/g, "").trim())
            .filter(Boolean);

        if (!clauses.length) {
            return false;
        }

        return clauses.every((clause) => evaluateMediaClause(clause, viewportState));
    }

    function evaluateMediaClause(clause, viewportState) {
        if (clause.startsWith("min-width:")) {
            return viewportState.innerWidth >= parsePixels(clause.slice("min-width:".length));
        }
        if (clause.startsWith("max-width:")) {
            return viewportState.innerWidth <= parsePixels(clause.slice("max-width:".length));
        }
        if (clause.startsWith("min-height:")) {
            return viewportState.innerHeight >= parsePixels(clause.slice("min-height:".length));
        }
        if (clause.startsWith("max-height:")) {
            return viewportState.innerHeight <= parsePixels(clause.slice("max-height:".length));
        }
        if (clause.startsWith("orientation:")) {
            return clause.slice("orientation:".length).trim() === "landscape";
        }
        if (clause.startsWith("prefers-color-scheme:")) {
            return clause.slice("prefers-color-scheme:".length).trim() === "light";
        }
        if (clause.startsWith("prefers-reduced-motion:")) {
            return clause.slice("prefers-reduced-motion:".length).trim() === "no-preference";
        }
        return false;
    }

    function parsePixels(value) {
        return Number.parseFloat(String(value).replace(/px$/i, "").trim()) || 0;
    }
