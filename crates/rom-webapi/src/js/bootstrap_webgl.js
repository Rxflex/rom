    class WebGLShaderPrecisionFormat {
        constructor(rangeMin, rangeMax, precision) {
            this.rangeMin = rangeMin;
            this.rangeMax = rangeMax;
            this.precision = precision;
        }
    }

    class WebGLRenderingContext {}

    class WebGL2RenderingContext extends WebGLRenderingContext {}

    const WEBGL_CONSTANTS = {
        VENDOR: 0x1f00,
        RENDERER: 0x1f01,
        VERSION: 0x1f02,
        MAX_TEXTURE_SIZE: 0x0d33,
        MAX_VIEWPORT_DIMS: 0x0d3a,
        RED_BITS: 0x0d52,
        GREEN_BITS: 0x0d53,
        BLUE_BITS: 0x0d54,
        ALPHA_BITS: 0x0d55,
        DEPTH_BITS: 0x0d56,
        STENCIL_BITS: 0x0d57,
        ALIASED_POINT_SIZE_RANGE: 0x846d,
        ALIASED_LINE_WIDTH_RANGE: 0x846e,
        SHADING_LANGUAGE_VERSION: 0x8b8c,
        CURRENT_PROGRAM: 0x8b8d,
        ARRAY_BUFFER_BINDING: 0x8894,
        ELEMENT_ARRAY_BUFFER_BINDING: 0x8895,
        MAX_RENDERBUFFER_SIZE: 0x84e8,
        MAX_VERTEX_ATTRIBS: 0x8869,
        RGBA: 0x1908,
        UNSIGNED_BYTE: 0x1401,
        FLOAT: 0x1406,
        HIGH_FLOAT: 0x8df2,
        MEDIUM_FLOAT: 0x8df1,
        LOW_FLOAT: 0x8df0,
        HIGH_INT: 0x8df5,
        MEDIUM_INT: 0x8df4,
        LOW_INT: 0x8df3,
        FRAGMENT_SHADER: 0x8b30,
        VERTEX_SHADER: 0x8b31,
        COMPILE_STATUS: 0x8b81,
        LINK_STATUS: 0x8b82,
        ARRAY_BUFFER: 0x8892,
        ELEMENT_ARRAY_BUFFER: 0x8893,
        STATIC_DRAW: 0x88e4,
        TRIANGLES: 0x0004,
        TRIANGLE_STRIP: 0x0005,
        TEXTURE_2D: 0x0de1,
        TEXTURE0: 0x84c0,
        TEXTURE_MIN_FILTER: 0x2801,
        TEXTURE_MAG_FILTER: 0x2800,
        LINEAR: 0x2601,
        COLOR_BUFFER_BIT: 0x4000,
        DEPTH_BUFFER_BIT: 0x0100,
        STENCIL_BUFFER_BIT: 0x0400,
    };

    const WEBGL_DEBUG_RENDERER_INFO = {
        UNMASKED_VENDOR_WEBGL: 0x9245,
        UNMASKED_RENDERER_WEBGL: 0x9246,
    };

    const WEBGL_EXTENSIONS = [
        "ANGLE_instanced_arrays",
        "EXT_blend_minmax",
        "EXT_color_buffer_half_float",
        "EXT_float_blend",
        "EXT_frag_depth",
        "EXT_shader_texture_lod",
        "EXT_texture_compression_bptc",
        "EXT_texture_filter_anisotropic",
        "OES_element_index_uint",
        "OES_standard_derivatives",
        "OES_texture_float",
        "OES_texture_float_linear",
        "OES_texture_half_float",
        "OES_texture_half_float_linear",
        "WEBGL_compressed_texture_s3tc",
        "WEBGL_debug_renderer_info",
        "WEBGL_debug_shaders",
        "WEBGL_lose_context",
    ];

    function normalizeCanvasContextKind(kind) {
        const normalized = String(kind ?? "").toLowerCase();
        if (normalized === "experimental-webgl") {
            return "webgl";
        }
        return normalized;
    }

    function createWebGLEntropyProfile(version) {
        const platform = String(navigatorConfig.platform ?? "Win32");
        const language = String(navigatorConfig.language ?? "en-US");
        const renderer = platform.startsWith("Mac")
            ? "ANGLE Metal Renderer: Apple M2 Pro"
            : "ANGLE (Intel, Intel(R) UHD Graphics 620 Direct3D11 vs_5_0 ps_5_0, D3D11)";
        const vendor = platform.startsWith("Mac") ? "Apple Inc." : "Google Inc. (Intel)";

        let seed = 2166136261 >>> 0;
        seed = hashCanvasValue(seed, {
            version,
            platform,
            language,
            languages: navigatorConfig.languages ?? [],
            userAgent: navigatorConfig.userAgent ?? "",
            hardwareConcurrency: navigatorConfig.hardwareConcurrency ?? 0,
            deviceMemory: navigatorConfig.deviceMemory ?? 0,
        });

        return {
            seed: finalizeCanvasSeed(seed),
            version,
            vendor: "WebKit",
            renderer: "WebKit WebGL",
            unmaskedVendor: vendor,
            unmaskedRenderer: renderer,
            shadingLanguageVersion: `WebGL GLSL ES ${version === 2 ? "3.00" : "1.00"} (OpenGL ES GLSL ES ${version === 2 ? "3.0 Chromium" : "1.0 Chromium"})`,
            versionString: `WebGL ${version === 2 ? "2.0" : "1.0"} (OpenGL ES ${version === 2 ? "3.0" : "2.0"} Chromium)`,
        };
    }

    function createWebGLState(canvas, version) {
        const bitmapState = ensureCanvasBitmapState(canvas);
        const profile = createWebGLEntropyProfile(version);
        return {
            version,
            profile,
            seed: finalizeCanvasSeed(bitmapState.seed ^ profile.seed),
            clearColor: [0, 0, 0, 0],
            viewport: [0, 0, bitmapState.width, bitmapState.height],
            currentProgram: null,
            boundArrayBuffer: null,
            boundElementArrayBuffer: null,
            boundTexture2D: null,
            activeTexture: WEBGL_CONSTANTS.TEXTURE0,
            revisions: 0,
        };
    }

    function ensureWebGLState(context) {
        if (!context.__webglState) {
            context.__webglState = createWebGLState(context.canvas, context.__webglVersion);
        }
        return context.__webglState;
    }

    function appendWebGLOperation(context, type, details = {}) {
        const state = ensureWebGLState(context);
        state.revisions += 1;
        state.seed = finalizeCanvasSeed(hashCanvasValue(state.seed, { type, ...details, revision: state.revisions }));
        appendCanvasOperation(context.canvas, `webgl:${type}`, {
            version: state.version,
            ...details,
            revision: state.revisions,
        });
        return state;
    }

    function createWebGLObject(type) {
        return { __webglType: type, __webglId: Math.random().toString(36).slice(2) };
    }

    function fillWebGLPixels(state, x, y, width, height, pixels) {
        const total = Math.min(pixels.length, Math.max(0, width * height * 4));
        for (let index = 0; index < total; index += 4) {
            const pixelIndex = index / 4;
            const px = x + (pixelIndex % width);
            const py = y + Math.trunc(pixelIndex / width);
            const seed = finalizeCanvasSeed(hashCanvasNumbers(state.seed, px + 1, py + 1, state.revisions));
            pixels[index] = ((seed & 0xff) + Math.round(state.clearColor[0] * 255)) & 0xff;
            pixels[index + 1] = (((seed >>> 8) & 0xff) + Math.round(state.clearColor[1] * 255)) & 0xff;
            pixels[index + 2] = (((seed >>> 16) & 0xff) + Math.round(state.clearColor[2] * 255)) & 0xff;
            pixels[index + 3] = 255;
        }
        return pixels;
    }

    function webglParameterValue(context, parameter) {
        const state = ensureWebGLState(context);
        const bitmapState = ensureCanvasBitmapState(context.canvas);

        switch (parameter) {
            case WEBGL_CONSTANTS.VENDOR:
                return state.profile.vendor;
            case WEBGL_CONSTANTS.RENDERER:
                return state.profile.renderer;
            case WEBGL_CONSTANTS.VERSION:
                return state.profile.versionString;
            case WEBGL_CONSTANTS.SHADING_LANGUAGE_VERSION:
                return state.profile.shadingLanguageVersion;
            case WEBGL_DEBUG_RENDERER_INFO.UNMASKED_VENDOR_WEBGL:
                return state.profile.unmaskedVendor;
            case WEBGL_DEBUG_RENDERER_INFO.UNMASKED_RENDERER_WEBGL:
                return state.profile.unmaskedRenderer;
            case WEBGL_CONSTANTS.MAX_TEXTURE_SIZE:
            case WEBGL_CONSTANTS.MAX_RENDERBUFFER_SIZE:
                return 16384;
            case WEBGL_CONSTANTS.MAX_VERTEX_ATTRIBS:
                return 16;
            case WEBGL_CONSTANTS.MAX_VIEWPORT_DIMS:
                return new Int32Array([bitmapState.width, bitmapState.height]);
            case WEBGL_CONSTANTS.ALIASED_POINT_SIZE_RANGE:
                return new Float32Array([1, 1024]);
            case WEBGL_CONSTANTS.ALIASED_LINE_WIDTH_RANGE:
                return new Float32Array([1, 8]);
            case WEBGL_CONSTANTS.RED_BITS:
            case WEBGL_CONSTANTS.GREEN_BITS:
            case WEBGL_CONSTANTS.BLUE_BITS:
            case WEBGL_CONSTANTS.ALPHA_BITS:
                return 8;
            case WEBGL_CONSTANTS.DEPTH_BITS:
                return 24;
            case WEBGL_CONSTANTS.STENCIL_BITS:
                return 8;
            case WEBGL_CONSTANTS.CURRENT_PROGRAM:
                return state.currentProgram;
            case WEBGL_CONSTANTS.ARRAY_BUFFER_BINDING:
                return state.boundArrayBuffer;
            case WEBGL_CONSTANTS.ELEMENT_ARRAY_BUFFER_BINDING:
                return state.boundElementArrayBuffer;
            default:
                return null;
        }
    }

    function createWebGLContext(kind, canvas) {
        const normalizedKind = normalizeCanvasContextKind(kind);
        const version = normalizedKind === "webgl2" ? 2 : normalizedKind === "webgl" ? 1 : 0;
        if (version === 0) {
            return null;
        }

        const ContextClass = version === 2 ? WebGL2RenderingContext : WebGLRenderingContext;
        const context = Object.create(ContextClass.prototype);
        Object.assign(context, WEBGL_CONSTANTS, WEBGL_DEBUG_RENDERER_INFO, {
            canvas,
            drawingBufferWidth: canvas?.width ?? 300,
            drawingBufferHeight: canvas?.height ?? 150,
            __webglVersion: version,
            getContextAttributes() {
                return {
                    alpha: true,
                    antialias: true,
                    depth: true,
                    desynchronized: false,
                    failIfMajorPerformanceCaveat: false,
                    powerPreference: "default",
                    premultipliedAlpha: true,
                    preserveDrawingBuffer: false,
                    stencil: true,
                    xrCompatible: false,
                };
            },
            getSupportedExtensions() {
                return WEBGL_EXTENSIONS.slice();
            },
            getExtension(name) {
                const normalizedName = String(name ?? "");
                if (!WEBGL_EXTENSIONS.includes(normalizedName)) {
                    return null;
                }
                if (normalizedName === "WEBGL_debug_renderer_info") {
                    return { ...WEBGL_DEBUG_RENDERER_INFO };
                }
                return { name: normalizedName };
            },
            getParameter(parameter) {
                return webglParameterValue(this, parameter);
            },
            getShaderPrecisionFormat() {
                return new WebGLShaderPrecisionFormat(127, 127, 23);
            },
            createShader(type) {
                return { ...createWebGLObject("shader"), type, source: "", compiled: false };
            },
            shaderSource(shader, source) {
                shader.source = String(source ?? "");
                appendWebGLOperation(this, "shaderSource", { type: shader.type, length: shader.source.length });
            },
            compileShader(shader) {
                shader.compiled = shader.source.length > 0;
                appendWebGLOperation(this, "compileShader", { type: shader.type, compiled: shader.compiled });
            },
            getShaderParameter(shader, parameter) {
                return parameter === WEBGL_CONSTANTS.COMPILE_STATUS ? Boolean(shader?.compiled) : null;
            },
            createProgram() {
                return { ...createWebGLObject("program"), shaders: [], linked: false };
            },
            attachShader(program, shader) {
                program.shaders.push(shader);
                appendWebGLOperation(this, "attachShader", { shaderType: shader?.type ?? 0 });
            },
            linkProgram(program) {
                program.linked = program.shaders.every((shader) => shader?.compiled);
                appendWebGLOperation(this, "linkProgram", { linked: program.linked, shaderCount: program.shaders.length });
            },
            getProgramParameter(program, parameter) {
                return parameter === WEBGL_CONSTANTS.LINK_STATUS ? Boolean(program?.linked) : null;
            },
            useProgram(program) {
                ensureWebGLState(this).currentProgram = program ?? null;
                appendWebGLOperation(this, "useProgram", { linked: Boolean(program?.linked) });
            },
            createBuffer() {
                return createWebGLObject("buffer");
            },
            bindBuffer(target, buffer) {
                const state = ensureWebGLState(this);
                if (target === WEBGL_CONSTANTS.ARRAY_BUFFER) {
                    state.boundArrayBuffer = buffer ?? null;
                }
                if (target === WEBGL_CONSTANTS.ELEMENT_ARRAY_BUFFER) {
                    state.boundElementArrayBuffer = buffer ?? null;
                }
                appendWebGLOperation(this, "bindBuffer", { target, type: buffer?.__webglType ?? null });
            },
            bufferData(target, data, usage) {
                appendWebGLOperation(this, "bufferData", {
                    target,
                    usage,
                    length: Number(data?.byteLength ?? data?.length ?? data ?? 0),
                });
            },
            createTexture() {
                return createWebGLObject("texture");
            },
            activeTexture(texture) {
                ensureWebGLState(this).activeTexture = Number(texture) || WEBGL_CONSTANTS.TEXTURE0;
            },
            bindTexture(target, texture) {
                const state = ensureWebGLState(this);
                if (target === WEBGL_CONSTANTS.TEXTURE_2D) {
                    state.boundTexture2D = texture ?? null;
                }
                appendWebGLOperation(this, "bindTexture", { target, type: texture?.__webglType ?? null });
            },
            texParameteri(target, pname, param) {
                appendWebGLOperation(this, "texParameteri", { target, pname, param });
            },
            texImage2D(...args) {
                appendWebGLOperation(this, "texImage2D", {
                    signature: args.map((value) => typeof value).join(","),
                    argCount: args.length,
                });
            },
            viewport(x, y, width, height) {
                ensureWebGLState(this).viewport = [x, y, width, height].map((value) => normalizeCanvasInteger(value, 0));
                appendWebGLOperation(this, "viewport", { x, y, width, height });
            },
            clearColor(r, g, b, a) {
                ensureWebGLState(this).clearColor = [r, g, b, a].map((value) => Math.max(0, Math.min(1, Number(value) || 0)));
                appendWebGLOperation(this, "clearColor", { r, g, b, a });
            },
            clear(mask) {
                appendWebGLOperation(this, "clear", { mask });
            },
            drawArrays(mode, first, count) {
                appendWebGLOperation(this, "drawArrays", { mode, first, count });
            },
            readPixels(x, y, width, height, _format, _type, pixels) {
                fillWebGLPixels(
                    ensureWebGLState(this),
                    normalizeCanvasInteger(x, 0),
                    normalizeCanvasInteger(y, 0),
                    Math.max(0, normalizeCanvasInteger(width, 0)),
                    Math.max(0, normalizeCanvasInteger(height, 0)),
                    pixels,
                );
            },
        });

        return context;
    }

    function createCanvasContext(kind, canvas = null) {
        const webglContext = createWebGLContext(kind, canvas);
        if (webglContext !== null) {
            return webglContext;
        }
        return createCanvas2DContext(kind, canvas);
    }

    g.WebGLRenderingContext = WebGLRenderingContext;
    g.WebGL2RenderingContext = WebGL2RenderingContext;
    g.WebGLShaderPrecisionFormat = WebGLShaderPrecisionFormat;
