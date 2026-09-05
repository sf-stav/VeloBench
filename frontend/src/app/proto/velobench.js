/*eslint-disable block-scoped-var, id-length, no-control-regex, no-magic-numbers, no-prototype-builtins, no-redeclare, no-shadow, no-var, sort-vars*/
import * as $protobuf from "protobufjs/minimal";

// Common aliases
const $Reader = $protobuf.Reader, $Writer = $protobuf.Writer, $util = $protobuf.util;

// Exported root namespace
const $root = $protobuf.roots["default"] || ($protobuf.roots["default"] = {});

export const velobench = $root.velobench = (() => {

    /**
     * Namespace velobench.
     * @exports velobench
     * @namespace
     */
    const velobench = {};

    velobench.ChatRequest = (function() {

        /**
         * Properties of a ChatRequest.
         * @memberof velobench
         * @interface IChatRequest
         * @property {string|null} [providerId] ChatRequest providerId
         * @property {string|null} [model] ChatRequest model
         * @property {Array.<velobench.IChatMessage>|null} [messages] ChatRequest messages
         * @property {boolean|null} [reasoningEnabled] ChatRequest reasoningEnabled
         * @property {string|null} [reasoningEffort] ChatRequest reasoningEffort
         * @property {Array.<velobench.IParamOverride>|null} [overrides] ChatRequest overrides
         * @property {number|null} [maxStatsTokens] ChatRequest maxStatsTokens
         * @property {boolean|null} [resetSession] ChatRequest resetSession
         * @property {boolean|null} [resetStats] ChatRequest resetStats
         * @property {number|null} [fillTokens] ChatRequest fillTokens
         * @property {string|null} [modelUid] ChatRequest modelUid
         * @property {string|null} [kind] ChatRequest kind
         * @property {string|null} [label] ChatRequest label
         * @property {string|null} [session] ChatRequest session
         * @property {string|null} [section] ChatRequest section
         * @property {boolean|null} [regimesFromSections] ChatRequest regimesFromSections
         */

        /**
         * Constructs a new ChatRequest.
         * @memberof velobench
         * @classdesc Represents a ChatRequest.
         * @implements IChatRequest
         * @constructor
         * @param {velobench.IChatRequest=} [properties] Properties to set
         */
        function ChatRequest(properties) {
            this.messages = [];
            this.overrides = [];
            if (properties)
                for (let keys = Object.keys(properties), i = 0; i < keys.length; ++i)
                    if (properties[keys[i]] != null && keys[i] !== "__proto__")
                        this[keys[i]] = properties[keys[i]];
        }

        /**
         * ChatRequest providerId.
         * @member {string} providerId
         * @memberof velobench.ChatRequest
         * @instance
         */
        ChatRequest.prototype.providerId = "";

        /**
         * ChatRequest model.
         * @member {string} model
         * @memberof velobench.ChatRequest
         * @instance
         */
        ChatRequest.prototype.model = "";

        /**
         * ChatRequest messages.
         * @member {Array.<velobench.IChatMessage>} messages
         * @memberof velobench.ChatRequest
         * @instance
         */
        ChatRequest.prototype.messages = $util.emptyArray;

        /**
         * ChatRequest reasoningEnabled.
         * @member {boolean} reasoningEnabled
         * @memberof velobench.ChatRequest
         * @instance
         */
        ChatRequest.prototype.reasoningEnabled = false;

        /**
         * ChatRequest reasoningEffort.
         * @member {string} reasoningEffort
         * @memberof velobench.ChatRequest
         * @instance
         */
        ChatRequest.prototype.reasoningEffort = "";

        /**
         * ChatRequest overrides.
         * @member {Array.<velobench.IParamOverride>} overrides
         * @memberof velobench.ChatRequest
         * @instance
         */
        ChatRequest.prototype.overrides = $util.emptyArray;

        /**
         * ChatRequest maxStatsTokens.
         * @member {number} maxStatsTokens
         * @memberof velobench.ChatRequest
         * @instance
         */
        ChatRequest.prototype.maxStatsTokens = 0;

        /**
         * ChatRequest resetSession.
         * @member {boolean} resetSession
         * @memberof velobench.ChatRequest
         * @instance
         */
        ChatRequest.prototype.resetSession = false;

        /**
         * ChatRequest resetStats.
         * @member {boolean} resetStats
         * @memberof velobench.ChatRequest
         * @instance
         */
        ChatRequest.prototype.resetStats = false;

        /**
         * ChatRequest fillTokens.
         * @member {number} fillTokens
         * @memberof velobench.ChatRequest
         * @instance
         */
        ChatRequest.prototype.fillTokens = 0;

        /**
         * ChatRequest modelUid.
         * @member {string} modelUid
         * @memberof velobench.ChatRequest
         * @instance
         */
        ChatRequest.prototype.modelUid = "";

        /**
         * ChatRequest kind.
         * @member {string} kind
         * @memberof velobench.ChatRequest
         * @instance
         */
        ChatRequest.prototype.kind = "";

        /**
         * ChatRequest label.
         * @member {string} label
         * @memberof velobench.ChatRequest
         * @instance
         */
        ChatRequest.prototype.label = "";

        /**
         * ChatRequest session.
         * @member {string} session
         * @memberof velobench.ChatRequest
         * @instance
         */
        ChatRequest.prototype.session = "";

        /**
         * ChatRequest section.
         * @member {string} section
         * @memberof velobench.ChatRequest
         * @instance
         */
        ChatRequest.prototype.section = "";

        /**
         * ChatRequest regimesFromSections.
         * @member {boolean} regimesFromSections
         * @memberof velobench.ChatRequest
         * @instance
         */
        ChatRequest.prototype.regimesFromSections = false;

        /**
         * Creates a new ChatRequest instance using the specified properties.
         * @function create
         * @memberof velobench.ChatRequest
         * @static
         * @param {velobench.IChatRequest=} [properties] Properties to set
         * @returns {velobench.ChatRequest} ChatRequest instance
         */
        ChatRequest.create = function create(properties) {
            return new ChatRequest(properties);
        };

        /**
         * Encodes the specified ChatRequest message. Does not implicitly {@link velobench.ChatRequest.verify|verify} messages.
         * @function encode
         * @memberof velobench.ChatRequest
         * @static
         * @param {velobench.IChatRequest} message ChatRequest message or plain object to encode
         * @param {$protobuf.Writer} [writer] Writer to encode to
         * @returns {$protobuf.Writer} Writer
         */
        ChatRequest.encode = function encode(message, writer, q) {
            if (!writer)
                writer = $Writer.create();
            if (q === undefined)
                q = 0;
            if (q > $util.recursionLimit)
                throw Error("max depth exceeded");
            if (message.providerId != null && Object.hasOwnProperty.call(message, "providerId"))
                writer.uint32(/* id 1, wireType 2 =*/10).string(message.providerId);
            if (message.model != null && Object.hasOwnProperty.call(message, "model"))
                writer.uint32(/* id 2, wireType 2 =*/18).string(message.model);
            if (message.messages != null && message.messages.length)
                for (let i = 0; i < message.messages.length; ++i)
                    $root.velobench.ChatMessage.encode(message.messages[i], writer.uint32(/* id 3, wireType 2 =*/26).fork(), q + 1).ldelim();
            if (message.reasoningEnabled != null && Object.hasOwnProperty.call(message, "reasoningEnabled"))
                writer.uint32(/* id 4, wireType 0 =*/32).bool(message.reasoningEnabled);
            if (message.reasoningEffort != null && Object.hasOwnProperty.call(message, "reasoningEffort"))
                writer.uint32(/* id 5, wireType 2 =*/42).string(message.reasoningEffort);
            if (message.overrides != null && message.overrides.length)
                for (let i = 0; i < message.overrides.length; ++i)
                    $root.velobench.ParamOverride.encode(message.overrides[i], writer.uint32(/* id 6, wireType 2 =*/50).fork(), q + 1).ldelim();
            if (message.maxStatsTokens != null && Object.hasOwnProperty.call(message, "maxStatsTokens"))
                writer.uint32(/* id 7, wireType 1 =*/57).double(message.maxStatsTokens);
            if (message.resetSession != null && Object.hasOwnProperty.call(message, "resetSession"))
                writer.uint32(/* id 8, wireType 0 =*/64).bool(message.resetSession);
            if (message.kind != null && Object.hasOwnProperty.call(message, "kind"))
                writer.uint32(/* id 9, wireType 2 =*/74).string(message.kind);
            if (message.label != null && Object.hasOwnProperty.call(message, "label"))
                writer.uint32(/* id 10, wireType 2 =*/82).string(message.label);
            if (message.session != null && Object.hasOwnProperty.call(message, "session"))
                writer.uint32(/* id 11, wireType 2 =*/90).string(message.session);
            if (message.section != null && Object.hasOwnProperty.call(message, "section"))
                writer.uint32(/* id 12, wireType 2 =*/98).string(message.section);
            if (message.regimesFromSections != null && Object.hasOwnProperty.call(message, "regimesFromSections"))
                writer.uint32(/* id 13, wireType 0 =*/104).bool(message.regimesFromSections);
            if (message.resetStats != null && Object.hasOwnProperty.call(message, "resetStats"))
                writer.uint32(/* id 14, wireType 0 =*/112).bool(message.resetStats);
            if (message.fillTokens != null && Object.hasOwnProperty.call(message, "fillTokens"))
                writer.uint32(/* id 15, wireType 0 =*/120).uint32(message.fillTokens);
            if (message.modelUid != null && Object.hasOwnProperty.call(message, "modelUid"))
                writer.uint32(/* id 16, wireType 2 =*/130).string(message.modelUid);
            return writer;
        };

        /**
         * Encodes the specified ChatRequest message, length delimited. Does not implicitly {@link velobench.ChatRequest.verify|verify} messages.
         * @function encodeDelimited
         * @memberof velobench.ChatRequest
         * @static
         * @param {velobench.IChatRequest} message ChatRequest message or plain object to encode
         * @param {$protobuf.Writer} [writer] Writer to encode to
         * @returns {$protobuf.Writer} Writer
         */
        ChatRequest.encodeDelimited = function encodeDelimited(message, writer) {
            return this.encode(message, writer && writer.len ? writer.fork() : writer).ldelim();
        };

        /**
         * Decodes a ChatRequest message from the specified reader or buffer.
         * @function decode
         * @memberof velobench.ChatRequest
         * @static
         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
         * @param {number} [length] Message length if known beforehand
         * @returns {velobench.ChatRequest} ChatRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        ChatRequest.decode = function decode(reader, length, error, long) {
            if (!(reader instanceof $Reader))
                reader = $Reader.create(reader);
            if (long === undefined)
                long = 0;
            if (long > $Reader.recursionLimit)
                throw Error("maximum nesting depth exceeded");
            let end, message;
            if (length === undefined)
                end = reader.len;
            else {
                end = reader.pos + length;
                if (end > reader.len)
                    throw RangeError("index out of range");
                length = reader.len;
                reader.len = end;
            }
            message = new $root.velobench.ChatRequest();
            while (reader.pos < end) {
                let tag = reader.uint32();
                if (tag === error)
                    break;
                switch (tag >>> 3) {
                case 1: {
                        message.providerId = reader.string();
                        break;
                    }
                case 2: {
                        message.model = reader.string();
                        break;
                    }
                case 3: {
                        if (!(message.messages && message.messages.length))
                            message.messages = [];
                        message.messages.push($root.velobench.ChatMessage.decode(reader, reader.uint32(), undefined, long + 1));
                        break;
                    }
                case 4: {
                        message.reasoningEnabled = reader.bool();
                        break;
                    }
                case 5: {
                        message.reasoningEffort = reader.string();
                        break;
                    }
                case 6: {
                        if (!(message.overrides && message.overrides.length))
                            message.overrides = [];
                        message.overrides.push($root.velobench.ParamOverride.decode(reader, reader.uint32(), undefined, long + 1));
                        break;
                    }
                case 7: {
                        message.maxStatsTokens = reader.double();
                        break;
                    }
                case 8: {
                        message.resetSession = reader.bool();
                        break;
                    }
                case 14: {
                        message.resetStats = reader.bool();
                        break;
                    }
                case 15: {
                        message.fillTokens = reader.uint32();
                        break;
                    }
                case 16: {
                        message.modelUid = reader.string();
                        break;
                    }
                case 9: {
                        message.kind = reader.string();
                        break;
                    }
                case 10: {
                        message.label = reader.string();
                        break;
                    }
                case 11: {
                        message.session = reader.string();
                        break;
                    }
                case 12: {
                        message.section = reader.string();
                        break;
                    }
                case 13: {
                        message.regimesFromSections = reader.bool();
                        break;
                    }
                default:
                    reader.skipType(tag & 7, long);
                    break;
                }
            }
            if (length !== undefined) {
                if (reader.pos !== end)
                    throw RangeError("index out of range");
                reader.len = length;
            }
            return message;
        };

        /**
         * Decodes a ChatRequest message from the specified reader or buffer, length delimited.
         * @function decodeDelimited
         * @memberof velobench.ChatRequest
         * @static
         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
         * @returns {velobench.ChatRequest} ChatRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        ChatRequest.decodeDelimited = function decodeDelimited(reader) {
            if (!(reader instanceof $Reader))
                reader = new $Reader(reader);
            return this.decode(reader, reader.uint32());
        };

        /**
         * Verifies a ChatRequest message.
         * @function verify
         * @memberof velobench.ChatRequest
         * @static
         * @param {Object.<string,*>} message Plain object to verify
         * @returns {string|null} `null` if valid, otherwise the reason why it is not
         */
        ChatRequest.verify = function verify(message, long) {
            if (typeof message !== "object" || message === null)
                return "object expected";
            if (long === undefined)
                long = 0;
            if (long > $util.recursionLimit)
                return "maximum nesting depth exceeded";
            if (message.providerId != null && Object.hasOwnProperty.call(message, "providerId"))
                if (!$util.isString(message.providerId))
                    return "providerId: string expected";
            if (message.model != null && Object.hasOwnProperty.call(message, "model"))
                if (!$util.isString(message.model))
                    return "model: string expected";
            if (message.messages != null && Object.hasOwnProperty.call(message, "messages")) {
                if (!Array.isArray(message.messages))
                    return "messages: array expected";
                for (let i = 0; i < message.messages.length; ++i) {
                    let error = $root.velobench.ChatMessage.verify(message.messages[i], long + 1);
                    if (error)
                        return "messages." + error;
                }
            }
            if (message.reasoningEnabled != null && Object.hasOwnProperty.call(message, "reasoningEnabled"))
                if (typeof message.reasoningEnabled !== "boolean")
                    return "reasoningEnabled: boolean expected";
            if (message.reasoningEffort != null && Object.hasOwnProperty.call(message, "reasoningEffort"))
                if (!$util.isString(message.reasoningEffort))
                    return "reasoningEffort: string expected";
            if (message.overrides != null && Object.hasOwnProperty.call(message, "overrides")) {
                if (!Array.isArray(message.overrides))
                    return "overrides: array expected";
                for (let i = 0; i < message.overrides.length; ++i) {
                    let error = $root.velobench.ParamOverride.verify(message.overrides[i], long + 1);
                    if (error)
                        return "overrides." + error;
                }
            }
            if (message.maxStatsTokens != null && Object.hasOwnProperty.call(message, "maxStatsTokens"))
                if (typeof message.maxStatsTokens !== "number")
                    return "maxStatsTokens: number expected";
            if (message.resetSession != null && Object.hasOwnProperty.call(message, "resetSession"))
                if (typeof message.resetSession !== "boolean")
                    return "resetSession: boolean expected";
            if (message.resetStats != null && Object.hasOwnProperty.call(message, "resetStats"))
                if (typeof message.resetStats !== "boolean")
                    return "resetStats: boolean expected";
            if (message.fillTokens != null && Object.hasOwnProperty.call(message, "fillTokens"))
                if (!$util.isInteger(message.fillTokens))
                    return "fillTokens: integer expected";
            if (message.modelUid != null && Object.hasOwnProperty.call(message, "modelUid"))
                if (!$util.isString(message.modelUid))
                    return "modelUid: string expected";
            if (message.kind != null && Object.hasOwnProperty.call(message, "kind"))
                if (!$util.isString(message.kind))
                    return "kind: string expected";
            if (message.label != null && Object.hasOwnProperty.call(message, "label"))
                if (!$util.isString(message.label))
                    return "label: string expected";
            if (message.session != null && Object.hasOwnProperty.call(message, "session"))
                if (!$util.isString(message.session))
                    return "session: string expected";
            if (message.section != null && Object.hasOwnProperty.call(message, "section"))
                if (!$util.isString(message.section))
                    return "section: string expected";
            if (message.regimesFromSections != null && Object.hasOwnProperty.call(message, "regimesFromSections"))
                if (typeof message.regimesFromSections !== "boolean")
                    return "regimesFromSections: boolean expected";
            return null;
        };

        /**
         * Creates a ChatRequest message from a plain object. Also converts values to their respective internal types.
         * @function fromObject
         * @memberof velobench.ChatRequest
         * @static
         * @param {Object.<string,*>} object Plain object
         * @returns {velobench.ChatRequest} ChatRequest
         */
        ChatRequest.fromObject = function fromObject(object, long) {
            if (object instanceof $root.velobench.ChatRequest)
                return object;
            if (!$util.isObject(object))
                throw TypeError(".velobench.ChatRequest: object expected");
            if (long === undefined)
                long = 0;
            if (long > $util.recursionLimit)
                throw Error("maximum nesting depth exceeded");
            let message = new $root.velobench.ChatRequest();
            if (object.providerId != null)
                message.providerId = String(object.providerId);
            if (object.model != null)
                message.model = String(object.model);
            if (object.messages) {
                if (!Array.isArray(object.messages))
                    throw TypeError(".velobench.ChatRequest.messages: array expected");
                message.messages = [];
                for (let i = 0; i < object.messages.length; ++i) {
                    if (!$util.isObject(object.messages[i]))
                        throw TypeError(".velobench.ChatRequest.messages: object expected");
                    message.messages[i] = $root.velobench.ChatMessage.fromObject(object.messages[i], long + 1);
                }
            }
            if (object.reasoningEnabled != null)
                message.reasoningEnabled = Boolean(object.reasoningEnabled);
            if (object.reasoningEffort != null)
                message.reasoningEffort = String(object.reasoningEffort);
            if (object.overrides) {
                if (!Array.isArray(object.overrides))
                    throw TypeError(".velobench.ChatRequest.overrides: array expected");
                message.overrides = [];
                for (let i = 0; i < object.overrides.length; ++i) {
                    if (!$util.isObject(object.overrides[i]))
                        throw TypeError(".velobench.ChatRequest.overrides: object expected");
                    message.overrides[i] = $root.velobench.ParamOverride.fromObject(object.overrides[i], long + 1);
                }
            }
            if (object.maxStatsTokens != null)
                message.maxStatsTokens = Number(object.maxStatsTokens);
            if (object.resetSession != null)
                message.resetSession = Boolean(object.resetSession);
            if (object.resetStats != null)
                message.resetStats = Boolean(object.resetStats);
            if (object.fillTokens != null)
                message.fillTokens = object.fillTokens >>> 0;
            if (object.modelUid != null)
                message.modelUid = String(object.modelUid);
            if (object.kind != null)
                message.kind = String(object.kind);
            if (object.label != null)
                message.label = String(object.label);
            if (object.session != null)
                message.session = String(object.session);
            if (object.section != null)
                message.section = String(object.section);
            if (object.regimesFromSections != null)
                message.regimesFromSections = Boolean(object.regimesFromSections);
            return message;
        };

        /**
         * Creates a plain object from a ChatRequest message. Also converts values to other types if specified.
         * @function toObject
         * @memberof velobench.ChatRequest
         * @static
         * @param {velobench.ChatRequest} message ChatRequest
         * @param {$protobuf.IConversionOptions} [options] Conversion options
         * @returns {Object.<string,*>} Plain object
         */
        ChatRequest.toObject = function toObject(message, options, q) {
            if (!options)
                options = {};
            if (q === undefined)
                q = 0;
            if (q > $util.recursionLimit)
                throw Error("max depth exceeded");
            let object = {};
            if (options.arrays || options.defaults) {
                object.messages = [];
                object.overrides = [];
            }
            if (options.defaults) {
                object.providerId = "";
                object.model = "";
                object.reasoningEnabled = false;
                object.reasoningEffort = "";
                object.maxStatsTokens = 0;
                object.resetSession = false;
                object.kind = "";
                object.label = "";
                object.session = "";
                object.section = "";
                object.regimesFromSections = false;
                object.resetStats = false;
                object.fillTokens = 0;
                object.modelUid = "";
            }
            if (message.providerId != null && Object.hasOwnProperty.call(message, "providerId"))
                object.providerId = message.providerId;
            if (message.model != null && Object.hasOwnProperty.call(message, "model"))
                object.model = message.model;
            if (message.messages && message.messages.length) {
                object.messages = [];
                for (let j = 0; j < message.messages.length; ++j)
                    object.messages[j] = $root.velobench.ChatMessage.toObject(message.messages[j], options, q + 1);
            }
            if (message.reasoningEnabled != null && Object.hasOwnProperty.call(message, "reasoningEnabled"))
                object.reasoningEnabled = message.reasoningEnabled;
            if (message.reasoningEffort != null && Object.hasOwnProperty.call(message, "reasoningEffort"))
                object.reasoningEffort = message.reasoningEffort;
            if (message.overrides && message.overrides.length) {
                object.overrides = [];
                for (let j = 0; j < message.overrides.length; ++j)
                    object.overrides[j] = $root.velobench.ParamOverride.toObject(message.overrides[j], options, q + 1);
            }
            if (message.maxStatsTokens != null && Object.hasOwnProperty.call(message, "maxStatsTokens"))
                object.maxStatsTokens = options.json && !isFinite(message.maxStatsTokens) ? String(message.maxStatsTokens) : message.maxStatsTokens;
            if (message.resetSession != null && Object.hasOwnProperty.call(message, "resetSession"))
                object.resetSession = message.resetSession;
            if (message.kind != null && Object.hasOwnProperty.call(message, "kind"))
                object.kind = message.kind;
            if (message.label != null && Object.hasOwnProperty.call(message, "label"))
                object.label = message.label;
            if (message.session != null && Object.hasOwnProperty.call(message, "session"))
                object.session = message.session;
            if (message.section != null && Object.hasOwnProperty.call(message, "section"))
                object.section = message.section;
            if (message.regimesFromSections != null && Object.hasOwnProperty.call(message, "regimesFromSections"))
                object.regimesFromSections = message.regimesFromSections;
            if (message.resetStats != null && Object.hasOwnProperty.call(message, "resetStats"))
                object.resetStats = message.resetStats;
            if (message.fillTokens != null && Object.hasOwnProperty.call(message, "fillTokens"))
                object.fillTokens = message.fillTokens;
            if (message.modelUid != null && Object.hasOwnProperty.call(message, "modelUid"))
                object.modelUid = message.modelUid;
            return object;
        };

        /**
         * Converts this ChatRequest to JSON.
         * @function toJSON
         * @memberof velobench.ChatRequest
         * @instance
         * @returns {Object.<string,*>} JSON object
         */
        ChatRequest.prototype.toJSON = function toJSON() {
            return this.constructor.toObject(this, $protobuf.util.toJSONOptions);
        };

        /**
         * Gets the default type url for ChatRequest
         * @function getTypeUrl
         * @memberof velobench.ChatRequest
         * @static
         * @param {string} [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns {string} The default type url
         */
        ChatRequest.getTypeUrl = function getTypeUrl(typeUrlPrefix) {
            if (typeUrlPrefix === undefined) {
                typeUrlPrefix = "type.googleapis.com";
            }
            return typeUrlPrefix + "/velobench.ChatRequest";
        };

        return ChatRequest;
    })();

    velobench.ChatMessage = (function() {

        /**
         * Properties of a ChatMessage.
         * @memberof velobench
         * @interface IChatMessage
         * @property {string|null} [role] ChatMessage role
         * @property {string|null} [content] ChatMessage content
         * @property {Array.<string>|null} [images] ChatMessage images
         * @property {number|null} [fillTokens] ChatMessage fillTokens
         */

        /**
         * Constructs a new ChatMessage.
         * @memberof velobench
         * @classdesc Represents a ChatMessage.
         * @implements IChatMessage
         * @constructor
         * @param {velobench.IChatMessage=} [properties] Properties to set
         */
        function ChatMessage(properties) {
            this.images = [];
            if (properties)
                for (let keys = Object.keys(properties), i = 0; i < keys.length; ++i)
                    if (properties[keys[i]] != null && keys[i] !== "__proto__")
                        this[keys[i]] = properties[keys[i]];
        }

        /**
         * ChatMessage role.
         * @member {string} role
         * @memberof velobench.ChatMessage
         * @instance
         */
        ChatMessage.prototype.role = "";

        /**
         * ChatMessage content.
         * @member {string} content
         * @memberof velobench.ChatMessage
         * @instance
         */
        ChatMessage.prototype.content = "";

        /**
         * ChatMessage images.
         * @member {Array.<string>} images
         * @memberof velobench.ChatMessage
         * @instance
         */
        ChatMessage.prototype.images = $util.emptyArray;

        /**
         * ChatMessage fillTokens.
         * @member {number} fillTokens
         * @memberof velobench.ChatMessage
         * @instance
         */
        ChatMessage.prototype.fillTokens = 0;

        /**
         * Creates a new ChatMessage instance using the specified properties.
         * @function create
         * @memberof velobench.ChatMessage
         * @static
         * @param {velobench.IChatMessage=} [properties] Properties to set
         * @returns {velobench.ChatMessage} ChatMessage instance
         */
        ChatMessage.create = function create(properties) {
            return new ChatMessage(properties);
        };

        /**
         * Encodes the specified ChatMessage message. Does not implicitly {@link velobench.ChatMessage.verify|verify} messages.
         * @function encode
         * @memberof velobench.ChatMessage
         * @static
         * @param {velobench.IChatMessage} message ChatMessage message or plain object to encode
         * @param {$protobuf.Writer} [writer] Writer to encode to
         * @returns {$protobuf.Writer} Writer
         */
        ChatMessage.encode = function encode(message, writer, q) {
            if (!writer)
                writer = $Writer.create();
            if (q === undefined)
                q = 0;
            if (q > $util.recursionLimit)
                throw Error("max depth exceeded");
            if (message.role != null && Object.hasOwnProperty.call(message, "role"))
                writer.uint32(/* id 1, wireType 2 =*/10).string(message.role);
            if (message.content != null && Object.hasOwnProperty.call(message, "content"))
                writer.uint32(/* id 2, wireType 2 =*/18).string(message.content);
            if (message.images != null && message.images.length)
                for (let i = 0; i < message.images.length; ++i)
                    writer.uint32(/* id 3, wireType 2 =*/26).string(message.images[i]);
            if (message.fillTokens != null && Object.hasOwnProperty.call(message, "fillTokens"))
                writer.uint32(/* id 4, wireType 0 =*/32).uint32(message.fillTokens);
            return writer;
        };

        /**
         * Encodes the specified ChatMessage message, length delimited. Does not implicitly {@link velobench.ChatMessage.verify|verify} messages.
         * @function encodeDelimited
         * @memberof velobench.ChatMessage
         * @static
         * @param {velobench.IChatMessage} message ChatMessage message or plain object to encode
         * @param {$protobuf.Writer} [writer] Writer to encode to
         * @returns {$protobuf.Writer} Writer
         */
        ChatMessage.encodeDelimited = function encodeDelimited(message, writer) {
            return this.encode(message, writer && writer.len ? writer.fork() : writer).ldelim();
        };

        /**
         * Decodes a ChatMessage message from the specified reader or buffer.
         * @function decode
         * @memberof velobench.ChatMessage
         * @static
         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
         * @param {number} [length] Message length if known beforehand
         * @returns {velobench.ChatMessage} ChatMessage
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        ChatMessage.decode = function decode(reader, length, error, long) {
            if (!(reader instanceof $Reader))
                reader = $Reader.create(reader);
            if (long === undefined)
                long = 0;
            if (long > $Reader.recursionLimit)
                throw Error("maximum nesting depth exceeded");
            let end, message;
            if (length === undefined)
                end = reader.len;
            else {
                end = reader.pos + length;
                if (end > reader.len)
                    throw RangeError("index out of range");
                length = reader.len;
                reader.len = end;
            }
            message = new $root.velobench.ChatMessage();
            while (reader.pos < end) {
                let tag = reader.uint32();
                if (tag === error)
                    break;
                switch (tag >>> 3) {
                case 1: {
                        message.role = reader.string();
                        break;
                    }
                case 2: {
                        message.content = reader.string();
                        break;
                    }
                case 3: {
                        if (!(message.images && message.images.length))
                            message.images = [];
                        message.images.push(reader.string());
                        break;
                    }
                case 4: {
                        message.fillTokens = reader.uint32();
                        break;
                    }
                default:
                    reader.skipType(tag & 7, long);
                    break;
                }
            }
            if (length !== undefined) {
                if (reader.pos !== end)
                    throw RangeError("index out of range");
                reader.len = length;
            }
            return message;
        };

        /**
         * Decodes a ChatMessage message from the specified reader or buffer, length delimited.
         * @function decodeDelimited
         * @memberof velobench.ChatMessage
         * @static
         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
         * @returns {velobench.ChatMessage} ChatMessage
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        ChatMessage.decodeDelimited = function decodeDelimited(reader) {
            if (!(reader instanceof $Reader))
                reader = new $Reader(reader);
            return this.decode(reader, reader.uint32());
        };

        /**
         * Verifies a ChatMessage message.
         * @function verify
         * @memberof velobench.ChatMessage
         * @static
         * @param {Object.<string,*>} message Plain object to verify
         * @returns {string|null} `null` if valid, otherwise the reason why it is not
         */
        ChatMessage.verify = function verify(message, long) {
            if (typeof message !== "object" || message === null)
                return "object expected";
            if (long === undefined)
                long = 0;
            if (long > $util.recursionLimit)
                return "maximum nesting depth exceeded";
            if (message.role != null && Object.hasOwnProperty.call(message, "role"))
                if (!$util.isString(message.role))
                    return "role: string expected";
            if (message.content != null && Object.hasOwnProperty.call(message, "content"))
                if (!$util.isString(message.content))
                    return "content: string expected";
            if (message.images != null && Object.hasOwnProperty.call(message, "images")) {
                if (!Array.isArray(message.images))
                    return "images: array expected";
                for (let i = 0; i < message.images.length; ++i)
                    if (!$util.isString(message.images[i]))
                        return "images: string[] expected";
            }
            if (message.fillTokens != null && Object.hasOwnProperty.call(message, "fillTokens"))
                if (!$util.isInteger(message.fillTokens))
                    return "fillTokens: integer expected";
            return null;
        };

        /**
         * Creates a ChatMessage message from a plain object. Also converts values to their respective internal types.
         * @function fromObject
         * @memberof velobench.ChatMessage
         * @static
         * @param {Object.<string,*>} object Plain object
         * @returns {velobench.ChatMessage} ChatMessage
         */
        ChatMessage.fromObject = function fromObject(object, long) {
            if (object instanceof $root.velobench.ChatMessage)
                return object;
            if (!$util.isObject(object))
                throw TypeError(".velobench.ChatMessage: object expected");
            if (long === undefined)
                long = 0;
            if (long > $util.recursionLimit)
                throw Error("maximum nesting depth exceeded");
            let message = new $root.velobench.ChatMessage();
            if (object.role != null)
                message.role = String(object.role);
            if (object.content != null)
                message.content = String(object.content);
            if (object.images) {
                if (!Array.isArray(object.images))
                    throw TypeError(".velobench.ChatMessage.images: array expected");
                message.images = [];
                for (let i = 0; i < object.images.length; ++i)
                    message.images[i] = String(object.images[i]);
            }
            if (object.fillTokens != null)
                message.fillTokens = object.fillTokens >>> 0;
            return message;
        };

        /**
         * Creates a plain object from a ChatMessage message. Also converts values to other types if specified.
         * @function toObject
         * @memberof velobench.ChatMessage
         * @static
         * @param {velobench.ChatMessage} message ChatMessage
         * @param {$protobuf.IConversionOptions} [options] Conversion options
         * @returns {Object.<string,*>} Plain object
         */
        ChatMessage.toObject = function toObject(message, options, q) {
            if (!options)
                options = {};
            if (q === undefined)
                q = 0;
            if (q > $util.recursionLimit)
                throw Error("max depth exceeded");
            let object = {};
            if (options.arrays || options.defaults)
                object.images = [];
            if (options.defaults) {
                object.role = "";
                object.content = "";
                object.fillTokens = 0;
            }
            if (message.role != null && Object.hasOwnProperty.call(message, "role"))
                object.role = message.role;
            if (message.content != null && Object.hasOwnProperty.call(message, "content"))
                object.content = message.content;
            if (message.images && message.images.length) {
                object.images = [];
                for (let j = 0; j < message.images.length; ++j)
                    object.images[j] = message.images[j];
            }
            if (message.fillTokens != null && Object.hasOwnProperty.call(message, "fillTokens"))
                object.fillTokens = message.fillTokens;
            return object;
        };

        /**
         * Converts this ChatMessage to JSON.
         * @function toJSON
         * @memberof velobench.ChatMessage
         * @instance
         * @returns {Object.<string,*>} JSON object
         */
        ChatMessage.prototype.toJSON = function toJSON() {
            return this.constructor.toObject(this, $protobuf.util.toJSONOptions);
        };

        /**
         * Gets the default type url for ChatMessage
         * @function getTypeUrl
         * @memberof velobench.ChatMessage
         * @static
         * @param {string} [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns {string} The default type url
         */
        ChatMessage.getTypeUrl = function getTypeUrl(typeUrlPrefix) {
            if (typeUrlPrefix === undefined) {
                typeUrlPrefix = "type.googleapis.com";
            }
            return typeUrlPrefix + "/velobench.ChatMessage";
        };

        return ChatMessage;
    })();

    velobench.ParamOverride = (function() {

        /**
         * Properties of a ParamOverride.
         * @memberof velobench
         * @interface IParamOverride
         * @property {string|null} [key] ParamOverride key
         * @property {string|null} [value] ParamOverride value
         */

        /**
         * Constructs a new ParamOverride.
         * @memberof velobench
         * @classdesc Represents a ParamOverride.
         * @implements IParamOverride
         * @constructor
         * @param {velobench.IParamOverride=} [properties] Properties to set
         */
        function ParamOverride(properties) {
            if (properties)
                for (let keys = Object.keys(properties), i = 0; i < keys.length; ++i)
                    if (properties[keys[i]] != null && keys[i] !== "__proto__")
                        this[keys[i]] = properties[keys[i]];
        }

        /**
         * ParamOverride key.
         * @member {string} key
         * @memberof velobench.ParamOverride
         * @instance
         */
        ParamOverride.prototype.key = "";

        /**
         * ParamOverride value.
         * @member {string} value
         * @memberof velobench.ParamOverride
         * @instance
         */
        ParamOverride.prototype.value = "";

        /**
         * Creates a new ParamOverride instance using the specified properties.
         * @function create
         * @memberof velobench.ParamOverride
         * @static
         * @param {velobench.IParamOverride=} [properties] Properties to set
         * @returns {velobench.ParamOverride} ParamOverride instance
         */
        ParamOverride.create = function create(properties) {
            return new ParamOverride(properties);
        };

        /**
         * Encodes the specified ParamOverride message. Does not implicitly {@link velobench.ParamOverride.verify|verify} messages.
         * @function encode
         * @memberof velobench.ParamOverride
         * @static
         * @param {velobench.IParamOverride} message ParamOverride message or plain object to encode
         * @param {$protobuf.Writer} [writer] Writer to encode to
         * @returns {$protobuf.Writer} Writer
         */
        ParamOverride.encode = function encode(message, writer, q) {
            if (!writer)
                writer = $Writer.create();
            if (q === undefined)
                q = 0;
            if (q > $util.recursionLimit)
                throw Error("max depth exceeded");
            if (message.key != null && Object.hasOwnProperty.call(message, "key"))
                writer.uint32(/* id 1, wireType 2 =*/10).string(message.key);
            if (message.value != null && Object.hasOwnProperty.call(message, "value"))
                writer.uint32(/* id 2, wireType 2 =*/18).string(message.value);
            return writer;
        };

        /**
         * Encodes the specified ParamOverride message, length delimited. Does not implicitly {@link velobench.ParamOverride.verify|verify} messages.
         * @function encodeDelimited
         * @memberof velobench.ParamOverride
         * @static
         * @param {velobench.IParamOverride} message ParamOverride message or plain object to encode
         * @param {$protobuf.Writer} [writer] Writer to encode to
         * @returns {$protobuf.Writer} Writer
         */
        ParamOverride.encodeDelimited = function encodeDelimited(message, writer) {
            return this.encode(message, writer && writer.len ? writer.fork() : writer).ldelim();
        };

        /**
         * Decodes a ParamOverride message from the specified reader or buffer.
         * @function decode
         * @memberof velobench.ParamOverride
         * @static
         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
         * @param {number} [length] Message length if known beforehand
         * @returns {velobench.ParamOverride} ParamOverride
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        ParamOverride.decode = function decode(reader, length, error, long) {
            if (!(reader instanceof $Reader))
                reader = $Reader.create(reader);
            if (long === undefined)
                long = 0;
            if (long > $Reader.recursionLimit)
                throw Error("maximum nesting depth exceeded");
            let end, message;
            if (length === undefined)
                end = reader.len;
            else {
                end = reader.pos + length;
                if (end > reader.len)
                    throw RangeError("index out of range");
                length = reader.len;
                reader.len = end;
            }
            message = new $root.velobench.ParamOverride();
            while (reader.pos < end) {
                let tag = reader.uint32();
                if (tag === error)
                    break;
                switch (tag >>> 3) {
                case 1: {
                        message.key = reader.string();
                        break;
                    }
                case 2: {
                        message.value = reader.string();
                        break;
                    }
                default:
                    reader.skipType(tag & 7, long);
                    break;
                }
            }
            if (length !== undefined) {
                if (reader.pos !== end)
                    throw RangeError("index out of range");
                reader.len = length;
            }
            return message;
        };

        /**
         * Decodes a ParamOverride message from the specified reader or buffer, length delimited.
         * @function decodeDelimited
         * @memberof velobench.ParamOverride
         * @static
         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
         * @returns {velobench.ParamOverride} ParamOverride
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        ParamOverride.decodeDelimited = function decodeDelimited(reader) {
            if (!(reader instanceof $Reader))
                reader = new $Reader(reader);
            return this.decode(reader, reader.uint32());
        };

        /**
         * Verifies a ParamOverride message.
         * @function verify
         * @memberof velobench.ParamOverride
         * @static
         * @param {Object.<string,*>} message Plain object to verify
         * @returns {string|null} `null` if valid, otherwise the reason why it is not
         */
        ParamOverride.verify = function verify(message, long) {
            if (typeof message !== "object" || message === null)
                return "object expected";
            if (long === undefined)
                long = 0;
            if (long > $util.recursionLimit)
                return "maximum nesting depth exceeded";
            if (message.key != null && Object.hasOwnProperty.call(message, "key"))
                if (!$util.isString(message.key))
                    return "key: string expected";
            if (message.value != null && Object.hasOwnProperty.call(message, "value"))
                if (!$util.isString(message.value))
                    return "value: string expected";
            return null;
        };

        /**
         * Creates a ParamOverride message from a plain object. Also converts values to their respective internal types.
         * @function fromObject
         * @memberof velobench.ParamOverride
         * @static
         * @param {Object.<string,*>} object Plain object
         * @returns {velobench.ParamOverride} ParamOverride
         */
        ParamOverride.fromObject = function fromObject(object, long) {
            if (object instanceof $root.velobench.ParamOverride)
                return object;
            if (!$util.isObject(object))
                throw TypeError(".velobench.ParamOverride: object expected");
            if (long === undefined)
                long = 0;
            if (long > $util.recursionLimit)
                throw Error("maximum nesting depth exceeded");
            let message = new $root.velobench.ParamOverride();
            if (object.key != null)
                message.key = String(object.key);
            if (object.value != null)
                message.value = String(object.value);
            return message;
        };

        /**
         * Creates a plain object from a ParamOverride message. Also converts values to other types if specified.
         * @function toObject
         * @memberof velobench.ParamOverride
         * @static
         * @param {velobench.ParamOverride} message ParamOverride
         * @param {$protobuf.IConversionOptions} [options] Conversion options
         * @returns {Object.<string,*>} Plain object
         */
        ParamOverride.toObject = function toObject(message, options, q) {
            if (!options)
                options = {};
            if (q === undefined)
                q = 0;
            if (q > $util.recursionLimit)
                throw Error("max depth exceeded");
            let object = {};
            if (options.defaults) {
                object.key = "";
                object.value = "";
            }
            if (message.key != null && Object.hasOwnProperty.call(message, "key"))
                object.key = message.key;
            if (message.value != null && Object.hasOwnProperty.call(message, "value"))
                object.value = message.value;
            return object;
        };

        /**
         * Converts this ParamOverride to JSON.
         * @function toJSON
         * @memberof velobench.ParamOverride
         * @instance
         * @returns {Object.<string,*>} JSON object
         */
        ParamOverride.prototype.toJSON = function toJSON() {
            return this.constructor.toObject(this, $protobuf.util.toJSONOptions);
        };

        /**
         * Gets the default type url for ParamOverride
         * @function getTypeUrl
         * @memberof velobench.ParamOverride
         * @static
         * @param {string} [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns {string} The default type url
         */
        ParamOverride.getTypeUrl = function getTypeUrl(typeUrlPrefix) {
            if (typeUrlPrefix === undefined) {
                typeUrlPrefix = "type.googleapis.com";
            }
            return typeUrlPrefix + "/velobench.ParamOverride";
        };

        return ParamOverride;
    })();

    velobench.ServerFrame = (function() {

        /**
         * Properties of a ServerFrame.
         * @memberof velobench
         * @interface IServerFrame
         * @property {velobench.IDelta|null} [delta] ServerFrame delta
         * @property {velobench.IStats|null} [stats] ServerFrame stats
         * @property {velobench.IDone|null} [done] ServerFrame done
         */

        /**
         * Constructs a new ServerFrame.
         * @memberof velobench
         * @classdesc Represents a ServerFrame.
         * @implements IServerFrame
         * @constructor
         * @param {velobench.IServerFrame=} [properties] Properties to set
         */
        function ServerFrame(properties) {
            if (properties)
                for (let keys = Object.keys(properties), i = 0; i < keys.length; ++i)
                    if (properties[keys[i]] != null && keys[i] !== "__proto__")
                        this[keys[i]] = properties[keys[i]];
        }

        /**
         * ServerFrame delta.
         * @member {velobench.IDelta|null|undefined} delta
         * @memberof velobench.ServerFrame
         * @instance
         */
        ServerFrame.prototype.delta = null;

        /**
         * ServerFrame stats.
         * @member {velobench.IStats|null|undefined} stats
         * @memberof velobench.ServerFrame
         * @instance
         */
        ServerFrame.prototype.stats = null;

        /**
         * ServerFrame done.
         * @member {velobench.IDone|null|undefined} done
         * @memberof velobench.ServerFrame
         * @instance
         */
        ServerFrame.prototype.done = null;

        // OneOf field names bound to virtual getters and setters
        let $oneOfFields;

        /**
         * ServerFrame payload.
         * @member {"delta"|"stats"|"done"|undefined} payload
         * @memberof velobench.ServerFrame
         * @instance
         */
        Object.defineProperty(ServerFrame.prototype, "payload", {
            get: $util.oneOfGetter($oneOfFields = ["delta", "stats", "done"]),
            set: $util.oneOfSetter($oneOfFields)
        });

        /**
         * Creates a new ServerFrame instance using the specified properties.
         * @function create
         * @memberof velobench.ServerFrame
         * @static
         * @param {velobench.IServerFrame=} [properties] Properties to set
         * @returns {velobench.ServerFrame} ServerFrame instance
         */
        ServerFrame.create = function create(properties) {
            return new ServerFrame(properties);
        };

        /**
         * Encodes the specified ServerFrame message. Does not implicitly {@link velobench.ServerFrame.verify|verify} messages.
         * @function encode
         * @memberof velobench.ServerFrame
         * @static
         * @param {velobench.IServerFrame} message ServerFrame message or plain object to encode
         * @param {$protobuf.Writer} [writer] Writer to encode to
         * @returns {$protobuf.Writer} Writer
         */
        ServerFrame.encode = function encode(message, writer, q) {
            if (!writer)
                writer = $Writer.create();
            if (q === undefined)
                q = 0;
            if (q > $util.recursionLimit)
                throw Error("max depth exceeded");
            if (message.delta != null && Object.hasOwnProperty.call(message, "delta"))
                $root.velobench.Delta.encode(message.delta, writer.uint32(/* id 1, wireType 2 =*/10).fork(), q + 1).ldelim();
            if (message.stats != null && Object.hasOwnProperty.call(message, "stats"))
                $root.velobench.Stats.encode(message.stats, writer.uint32(/* id 2, wireType 2 =*/18).fork(), q + 1).ldelim();
            if (message.done != null && Object.hasOwnProperty.call(message, "done"))
                $root.velobench.Done.encode(message.done, writer.uint32(/* id 3, wireType 2 =*/26).fork(), q + 1).ldelim();
            return writer;
        };

        /**
         * Encodes the specified ServerFrame message, length delimited. Does not implicitly {@link velobench.ServerFrame.verify|verify} messages.
         * @function encodeDelimited
         * @memberof velobench.ServerFrame
         * @static
         * @param {velobench.IServerFrame} message ServerFrame message or plain object to encode
         * @param {$protobuf.Writer} [writer] Writer to encode to
         * @returns {$protobuf.Writer} Writer
         */
        ServerFrame.encodeDelimited = function encodeDelimited(message, writer) {
            return this.encode(message, writer && writer.len ? writer.fork() : writer).ldelim();
        };

        /**
         * Decodes a ServerFrame message from the specified reader or buffer.
         * @function decode
         * @memberof velobench.ServerFrame
         * @static
         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
         * @param {number} [length] Message length if known beforehand
         * @returns {velobench.ServerFrame} ServerFrame
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        ServerFrame.decode = function decode(reader, length, error, long) {
            if (!(reader instanceof $Reader))
                reader = $Reader.create(reader);
            if (long === undefined)
                long = 0;
            if (long > $Reader.recursionLimit)
                throw Error("maximum nesting depth exceeded");
            let end, message;
            if (length === undefined)
                end = reader.len;
            else {
                end = reader.pos + length;
                if (end > reader.len)
                    throw RangeError("index out of range");
                length = reader.len;
                reader.len = end;
            }
            message = new $root.velobench.ServerFrame();
            while (reader.pos < end) {
                let tag = reader.uint32();
                if (tag === error)
                    break;
                switch (tag >>> 3) {
                case 1: {
                        message.delta = $root.velobench.Delta.decode(reader, reader.uint32(), undefined, long + 1);
                        break;
                    }
                case 2: {
                        message.stats = $root.velobench.Stats.decode(reader, reader.uint32(), undefined, long + 1);
                        break;
                    }
                case 3: {
                        message.done = $root.velobench.Done.decode(reader, reader.uint32(), undefined, long + 1);
                        break;
                    }
                default:
                    reader.skipType(tag & 7, long);
                    break;
                }
            }
            if (length !== undefined) {
                if (reader.pos !== end)
                    throw RangeError("index out of range");
                reader.len = length;
            }
            return message;
        };

        /**
         * Decodes a ServerFrame message from the specified reader or buffer, length delimited.
         * @function decodeDelimited
         * @memberof velobench.ServerFrame
         * @static
         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
         * @returns {velobench.ServerFrame} ServerFrame
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        ServerFrame.decodeDelimited = function decodeDelimited(reader) {
            if (!(reader instanceof $Reader))
                reader = new $Reader(reader);
            return this.decode(reader, reader.uint32());
        };

        /**
         * Verifies a ServerFrame message.
         * @function verify
         * @memberof velobench.ServerFrame
         * @static
         * @param {Object.<string,*>} message Plain object to verify
         * @returns {string|null} `null` if valid, otherwise the reason why it is not
         */
        ServerFrame.verify = function verify(message, long) {
            if (typeof message !== "object" || message === null)
                return "object expected";
            if (long === undefined)
                long = 0;
            if (long > $util.recursionLimit)
                return "maximum nesting depth exceeded";
            let properties = {};
            if (message.delta != null && Object.hasOwnProperty.call(message, "delta")) {
                properties.payload = 1;
                {
                    let error = $root.velobench.Delta.verify(message.delta, long + 1);
                    if (error)
                        return "delta." + error;
                }
            }
            if (message.stats != null && Object.hasOwnProperty.call(message, "stats")) {
                if (properties.payload === 1)
                    return "payload: multiple values";
                properties.payload = 1;
                {
                    let error = $root.velobench.Stats.verify(message.stats, long + 1);
                    if (error)
                        return "stats." + error;
                }
            }
            if (message.done != null && Object.hasOwnProperty.call(message, "done")) {
                if (properties.payload === 1)
                    return "payload: multiple values";
                properties.payload = 1;
                {
                    let error = $root.velobench.Done.verify(message.done, long + 1);
                    if (error)
                        return "done." + error;
                }
            }
            return null;
        };

        /**
         * Creates a ServerFrame message from a plain object. Also converts values to their respective internal types.
         * @function fromObject
         * @memberof velobench.ServerFrame
         * @static
         * @param {Object.<string,*>} object Plain object
         * @returns {velobench.ServerFrame} ServerFrame
         */
        ServerFrame.fromObject = function fromObject(object, long) {
            if (object instanceof $root.velobench.ServerFrame)
                return object;
            if (!$util.isObject(object))
                throw TypeError(".velobench.ServerFrame: object expected");
            if (long === undefined)
                long = 0;
            if (long > $util.recursionLimit)
                throw Error("maximum nesting depth exceeded");
            let message = new $root.velobench.ServerFrame();
            if (object.delta != null) {
                if (!$util.isObject(object.delta))
                    throw TypeError(".velobench.ServerFrame.delta: object expected");
                message.delta = $root.velobench.Delta.fromObject(object.delta, long + 1);
            }
            if (object.stats != null) {
                if (!$util.isObject(object.stats))
                    throw TypeError(".velobench.ServerFrame.stats: object expected");
                message.stats = $root.velobench.Stats.fromObject(object.stats, long + 1);
            }
            if (object.done != null) {
                if (!$util.isObject(object.done))
                    throw TypeError(".velobench.ServerFrame.done: object expected");
                message.done = $root.velobench.Done.fromObject(object.done, long + 1);
            }
            return message;
        };

        /**
         * Creates a plain object from a ServerFrame message. Also converts values to other types if specified.
         * @function toObject
         * @memberof velobench.ServerFrame
         * @static
         * @param {velobench.ServerFrame} message ServerFrame
         * @param {$protobuf.IConversionOptions} [options] Conversion options
         * @returns {Object.<string,*>} Plain object
         */
        ServerFrame.toObject = function toObject(message, options, q) {
            if (!options)
                options = {};
            if (q === undefined)
                q = 0;
            if (q > $util.recursionLimit)
                throw Error("max depth exceeded");
            let object = {};
            if (message.delta != null && Object.hasOwnProperty.call(message, "delta")) {
                object.delta = $root.velobench.Delta.toObject(message.delta, options, q + 1);
                if (options.oneofs)
                    object.payload = "delta";
            }
            if (message.stats != null && Object.hasOwnProperty.call(message, "stats")) {
                object.stats = $root.velobench.Stats.toObject(message.stats, options, q + 1);
                if (options.oneofs)
                    object.payload = "stats";
            }
            if (message.done != null && Object.hasOwnProperty.call(message, "done")) {
                object.done = $root.velobench.Done.toObject(message.done, options, q + 1);
                if (options.oneofs)
                    object.payload = "done";
            }
            return object;
        };

        /**
         * Converts this ServerFrame to JSON.
         * @function toJSON
         * @memberof velobench.ServerFrame
         * @instance
         * @returns {Object.<string,*>} JSON object
         */
        ServerFrame.prototype.toJSON = function toJSON() {
            return this.constructor.toObject(this, $protobuf.util.toJSONOptions);
        };

        /**
         * Gets the default type url for ServerFrame
         * @function getTypeUrl
         * @memberof velobench.ServerFrame
         * @static
         * @param {string} [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns {string} The default type url
         */
        ServerFrame.getTypeUrl = function getTypeUrl(typeUrlPrefix) {
            if (typeUrlPrefix === undefined) {
                typeUrlPrefix = "type.googleapis.com";
            }
            return typeUrlPrefix + "/velobench.ServerFrame";
        };

        return ServerFrame;
    })();

    velobench.Delta = (function() {

        /**
         * Properties of a Delta.
         * @memberof velobench
         * @interface IDelta
         * @property {string|null} [content] Delta content
         * @property {string|null} [reasoning] Delta reasoning
         */

        /**
         * Constructs a new Delta.
         * @memberof velobench
         * @classdesc Represents a Delta.
         * @implements IDelta
         * @constructor
         * @param {velobench.IDelta=} [properties] Properties to set
         */
        function Delta(properties) {
            if (properties)
                for (let keys = Object.keys(properties), i = 0; i < keys.length; ++i)
                    if (properties[keys[i]] != null && keys[i] !== "__proto__")
                        this[keys[i]] = properties[keys[i]];
        }

        /**
         * Delta content.
         * @member {string} content
         * @memberof velobench.Delta
         * @instance
         */
        Delta.prototype.content = "";

        /**
         * Delta reasoning.
         * @member {string} reasoning
         * @memberof velobench.Delta
         * @instance
         */
        Delta.prototype.reasoning = "";

        /**
         * Creates a new Delta instance using the specified properties.
         * @function create
         * @memberof velobench.Delta
         * @static
         * @param {velobench.IDelta=} [properties] Properties to set
         * @returns {velobench.Delta} Delta instance
         */
        Delta.create = function create(properties) {
            return new Delta(properties);
        };

        /**
         * Encodes the specified Delta message. Does not implicitly {@link velobench.Delta.verify|verify} messages.
         * @function encode
         * @memberof velobench.Delta
         * @static
         * @param {velobench.IDelta} message Delta message or plain object to encode
         * @param {$protobuf.Writer} [writer] Writer to encode to
         * @returns {$protobuf.Writer} Writer
         */
        Delta.encode = function encode(message, writer, q) {
            if (!writer)
                writer = $Writer.create();
            if (q === undefined)
                q = 0;
            if (q > $util.recursionLimit)
                throw Error("max depth exceeded");
            if (message.content != null && Object.hasOwnProperty.call(message, "content"))
                writer.uint32(/* id 1, wireType 2 =*/10).string(message.content);
            if (message.reasoning != null && Object.hasOwnProperty.call(message, "reasoning"))
                writer.uint32(/* id 2, wireType 2 =*/18).string(message.reasoning);
            return writer;
        };

        /**
         * Encodes the specified Delta message, length delimited. Does not implicitly {@link velobench.Delta.verify|verify} messages.
         * @function encodeDelimited
         * @memberof velobench.Delta
         * @static
         * @param {velobench.IDelta} message Delta message or plain object to encode
         * @param {$protobuf.Writer} [writer] Writer to encode to
         * @returns {$protobuf.Writer} Writer
         */
        Delta.encodeDelimited = function encodeDelimited(message, writer) {
            return this.encode(message, writer && writer.len ? writer.fork() : writer).ldelim();
        };

        /**
         * Decodes a Delta message from the specified reader or buffer.
         * @function decode
         * @memberof velobench.Delta
         * @static
         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
         * @param {number} [length] Message length if known beforehand
         * @returns {velobench.Delta} Delta
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        Delta.decode = function decode(reader, length, error, long) {
            if (!(reader instanceof $Reader))
                reader = $Reader.create(reader);
            if (long === undefined)
                long = 0;
            if (long > $Reader.recursionLimit)
                throw Error("maximum nesting depth exceeded");
            let end, message;
            if (length === undefined)
                end = reader.len;
            else {
                end = reader.pos + length;
                if (end > reader.len)
                    throw RangeError("index out of range");
                length = reader.len;
                reader.len = end;
            }
            message = new $root.velobench.Delta();
            while (reader.pos < end) {
                let tag = reader.uint32();
                if (tag === error)
                    break;
                switch (tag >>> 3) {
                case 1: {
                        message.content = reader.string();
                        break;
                    }
                case 2: {
                        message.reasoning = reader.string();
                        break;
                    }
                default:
                    reader.skipType(tag & 7, long);
                    break;
                }
            }
            if (length !== undefined) {
                if (reader.pos !== end)
                    throw RangeError("index out of range");
                reader.len = length;
            }
            return message;
        };

        /**
         * Decodes a Delta message from the specified reader or buffer, length delimited.
         * @function decodeDelimited
         * @memberof velobench.Delta
         * @static
         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
         * @returns {velobench.Delta} Delta
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        Delta.decodeDelimited = function decodeDelimited(reader) {
            if (!(reader instanceof $Reader))
                reader = new $Reader(reader);
            return this.decode(reader, reader.uint32());
        };

        /**
         * Verifies a Delta message.
         * @function verify
         * @memberof velobench.Delta
         * @static
         * @param {Object.<string,*>} message Plain object to verify
         * @returns {string|null} `null` if valid, otherwise the reason why it is not
         */
        Delta.verify = function verify(message, long) {
            if (typeof message !== "object" || message === null)
                return "object expected";
            if (long === undefined)
                long = 0;
            if (long > $util.recursionLimit)
                return "maximum nesting depth exceeded";
            if (message.content != null && Object.hasOwnProperty.call(message, "content"))
                if (!$util.isString(message.content))
                    return "content: string expected";
            if (message.reasoning != null && Object.hasOwnProperty.call(message, "reasoning"))
                if (!$util.isString(message.reasoning))
                    return "reasoning: string expected";
            return null;
        };

        /**
         * Creates a Delta message from a plain object. Also converts values to their respective internal types.
         * @function fromObject
         * @memberof velobench.Delta
         * @static
         * @param {Object.<string,*>} object Plain object
         * @returns {velobench.Delta} Delta
         */
        Delta.fromObject = function fromObject(object, long) {
            if (object instanceof $root.velobench.Delta)
                return object;
            if (!$util.isObject(object))
                throw TypeError(".velobench.Delta: object expected");
            if (long === undefined)
                long = 0;
            if (long > $util.recursionLimit)
                throw Error("maximum nesting depth exceeded");
            let message = new $root.velobench.Delta();
            if (object.content != null)
                message.content = String(object.content);
            if (object.reasoning != null)
                message.reasoning = String(object.reasoning);
            return message;
        };

        /**
         * Creates a plain object from a Delta message. Also converts values to other types if specified.
         * @function toObject
         * @memberof velobench.Delta
         * @static
         * @param {velobench.Delta} message Delta
         * @param {$protobuf.IConversionOptions} [options] Conversion options
         * @returns {Object.<string,*>} Plain object
         */
        Delta.toObject = function toObject(message, options, q) {
            if (!options)
                options = {};
            if (q === undefined)
                q = 0;
            if (q > $util.recursionLimit)
                throw Error("max depth exceeded");
            let object = {};
            if (options.defaults) {
                object.content = "";
                object.reasoning = "";
            }
            if (message.content != null && Object.hasOwnProperty.call(message, "content"))
                object.content = message.content;
            if (message.reasoning != null && Object.hasOwnProperty.call(message, "reasoning"))
                object.reasoning = message.reasoning;
            return object;
        };

        /**
         * Converts this Delta to JSON.
         * @function toJSON
         * @memberof velobench.Delta
         * @instance
         * @returns {Object.<string,*>} JSON object
         */
        Delta.prototype.toJSON = function toJSON() {
            return this.constructor.toObject(this, $protobuf.util.toJSONOptions);
        };

        /**
         * Gets the default type url for Delta
         * @function getTypeUrl
         * @memberof velobench.Delta
         * @static
         * @param {string} [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns {string} The default type url
         */
        Delta.getTypeUrl = function getTypeUrl(typeUrlPrefix) {
            if (typeUrlPrefix === undefined) {
                typeUrlPrefix = "type.googleapis.com";
            }
            return typeUrlPrefix + "/velobench.Delta";
        };

        return Delta;
    })();

    velobench.DecodePoint = (function() {

        /**
         * Properties of a DecodePoint.
         * @memberof velobench
         * @interface IDecodePoint
         * @property {number|null} [tMs] DecodePoint tMs
         * @property {number|null} [tokS] DecodePoint tokS
         * @property {string|null} [kind] DecodePoint kind
         * @property {string|null} [regime] DecodePoint regime
         */

        /**
         * Constructs a new DecodePoint.
         * @memberof velobench
         * @classdesc Represents a DecodePoint.
         * @implements IDecodePoint
         * @constructor
         * @param {velobench.IDecodePoint=} [properties] Properties to set
         */
        function DecodePoint(properties) {
            if (properties)
                for (let keys = Object.keys(properties), i = 0; i < keys.length; ++i)
                    if (properties[keys[i]] != null && keys[i] !== "__proto__")
                        this[keys[i]] = properties[keys[i]];
        }

        /**
         * DecodePoint tMs.
         * @member {number} tMs
         * @memberof velobench.DecodePoint
         * @instance
         */
        DecodePoint.prototype.tMs = 0;

        /**
         * DecodePoint tokS.
         * @member {number} tokS
         * @memberof velobench.DecodePoint
         * @instance
         */
        DecodePoint.prototype.tokS = 0;

        /**
         * DecodePoint kind.
         * @member {string} kind
         * @memberof velobench.DecodePoint
         * @instance
         */
        DecodePoint.prototype.kind = "";

        /**
         * DecodePoint regime.
         * @member {string} regime
         * @memberof velobench.DecodePoint
         * @instance
         */
        DecodePoint.prototype.regime = "";

        /**
         * Creates a new DecodePoint instance using the specified properties.
         * @function create
         * @memberof velobench.DecodePoint
         * @static
         * @param {velobench.IDecodePoint=} [properties] Properties to set
         * @returns {velobench.DecodePoint} DecodePoint instance
         */
        DecodePoint.create = function create(properties) {
            return new DecodePoint(properties);
        };

        /**
         * Encodes the specified DecodePoint message. Does not implicitly {@link velobench.DecodePoint.verify|verify} messages.
         * @function encode
         * @memberof velobench.DecodePoint
         * @static
         * @param {velobench.IDecodePoint} message DecodePoint message or plain object to encode
         * @param {$protobuf.Writer} [writer] Writer to encode to
         * @returns {$protobuf.Writer} Writer
         */
        DecodePoint.encode = function encode(message, writer, q) {
            if (!writer)
                writer = $Writer.create();
            if (q === undefined)
                q = 0;
            if (q > $util.recursionLimit)
                throw Error("max depth exceeded");
            if (message.tMs != null && Object.hasOwnProperty.call(message, "tMs"))
                writer.uint32(/* id 1, wireType 1 =*/9).double(message.tMs);
            if (message.tokS != null && Object.hasOwnProperty.call(message, "tokS"))
                writer.uint32(/* id 2, wireType 1 =*/17).double(message.tokS);
            if (message.kind != null && Object.hasOwnProperty.call(message, "kind"))
                writer.uint32(/* id 3, wireType 2 =*/26).string(message.kind);
            if (message.regime != null && Object.hasOwnProperty.call(message, "regime"))
                writer.uint32(/* id 4, wireType 2 =*/34).string(message.regime);
            return writer;
        };

        /**
         * Encodes the specified DecodePoint message, length delimited. Does not implicitly {@link velobench.DecodePoint.verify|verify} messages.
         * @function encodeDelimited
         * @memberof velobench.DecodePoint
         * @static
         * @param {velobench.IDecodePoint} message DecodePoint message or plain object to encode
         * @param {$protobuf.Writer} [writer] Writer to encode to
         * @returns {$protobuf.Writer} Writer
         */
        DecodePoint.encodeDelimited = function encodeDelimited(message, writer) {
            return this.encode(message, writer && writer.len ? writer.fork() : writer).ldelim();
        };

        /**
         * Decodes a DecodePoint message from the specified reader or buffer.
         * @function decode
         * @memberof velobench.DecodePoint
         * @static
         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
         * @param {number} [length] Message length if known beforehand
         * @returns {velobench.DecodePoint} DecodePoint
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        DecodePoint.decode = function decode(reader, length, error, long) {
            if (!(reader instanceof $Reader))
                reader = $Reader.create(reader);
            if (long === undefined)
                long = 0;
            if (long > $Reader.recursionLimit)
                throw Error("maximum nesting depth exceeded");
            let end, message;
            if (length === undefined)
                end = reader.len;
            else {
                end = reader.pos + length;
                if (end > reader.len)
                    throw RangeError("index out of range");
                length = reader.len;
                reader.len = end;
            }
            message = new $root.velobench.DecodePoint();
            while (reader.pos < end) {
                let tag = reader.uint32();
                if (tag === error)
                    break;
                switch (tag >>> 3) {
                case 1: {
                        message.tMs = reader.double();
                        break;
                    }
                case 2: {
                        message.tokS = reader.double();
                        break;
                    }
                case 3: {
                        message.kind = reader.string();
                        break;
                    }
                case 4: {
                        message.regime = reader.string();
                        break;
                    }
                default:
                    reader.skipType(tag & 7, long);
                    break;
                }
            }
            if (length !== undefined) {
                if (reader.pos !== end)
                    throw RangeError("index out of range");
                reader.len = length;
            }
            return message;
        };

        /**
         * Decodes a DecodePoint message from the specified reader or buffer, length delimited.
         * @function decodeDelimited
         * @memberof velobench.DecodePoint
         * @static
         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
         * @returns {velobench.DecodePoint} DecodePoint
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        DecodePoint.decodeDelimited = function decodeDelimited(reader) {
            if (!(reader instanceof $Reader))
                reader = new $Reader(reader);
            return this.decode(reader, reader.uint32());
        };

        /**
         * Verifies a DecodePoint message.
         * @function verify
         * @memberof velobench.DecodePoint
         * @static
         * @param {Object.<string,*>} message Plain object to verify
         * @returns {string|null} `null` if valid, otherwise the reason why it is not
         */
        DecodePoint.verify = function verify(message, long) {
            if (typeof message !== "object" || message === null)
                return "object expected";
            if (long === undefined)
                long = 0;
            if (long > $util.recursionLimit)
                return "maximum nesting depth exceeded";
            if (message.tMs != null && Object.hasOwnProperty.call(message, "tMs"))
                if (typeof message.tMs !== "number")
                    return "tMs: number expected";
            if (message.tokS != null && Object.hasOwnProperty.call(message, "tokS"))
                if (typeof message.tokS !== "number")
                    return "tokS: number expected";
            if (message.kind != null && Object.hasOwnProperty.call(message, "kind"))
                if (!$util.isString(message.kind))
                    return "kind: string expected";
            if (message.regime != null && Object.hasOwnProperty.call(message, "regime"))
                if (!$util.isString(message.regime))
                    return "regime: string expected";
            return null;
        };

        /**
         * Creates a DecodePoint message from a plain object. Also converts values to their respective internal types.
         * @function fromObject
         * @memberof velobench.DecodePoint
         * @static
         * @param {Object.<string,*>} object Plain object
         * @returns {velobench.DecodePoint} DecodePoint
         */
        DecodePoint.fromObject = function fromObject(object, long) {
            if (object instanceof $root.velobench.DecodePoint)
                return object;
            if (!$util.isObject(object))
                throw TypeError(".velobench.DecodePoint: object expected");
            if (long === undefined)
                long = 0;
            if (long > $util.recursionLimit)
                throw Error("maximum nesting depth exceeded");
            let message = new $root.velobench.DecodePoint();
            if (object.tMs != null)
                message.tMs = Number(object.tMs);
            if (object.tokS != null)
                message.tokS = Number(object.tokS);
            if (object.kind != null)
                message.kind = String(object.kind);
            if (object.regime != null)
                message.regime = String(object.regime);
            return message;
        };

        /**
         * Creates a plain object from a DecodePoint message. Also converts values to other types if specified.
         * @function toObject
         * @memberof velobench.DecodePoint
         * @static
         * @param {velobench.DecodePoint} message DecodePoint
         * @param {$protobuf.IConversionOptions} [options] Conversion options
         * @returns {Object.<string,*>} Plain object
         */
        DecodePoint.toObject = function toObject(message, options, q) {
            if (!options)
                options = {};
            if (q === undefined)
                q = 0;
            if (q > $util.recursionLimit)
                throw Error("max depth exceeded");
            let object = {};
            if (options.defaults) {
                object.tMs = 0;
                object.tokS = 0;
                object.kind = "";
                object.regime = "";
            }
            if (message.tMs != null && Object.hasOwnProperty.call(message, "tMs"))
                object.tMs = options.json && !isFinite(message.tMs) ? String(message.tMs) : message.tMs;
            if (message.tokS != null && Object.hasOwnProperty.call(message, "tokS"))
                object.tokS = options.json && !isFinite(message.tokS) ? String(message.tokS) : message.tokS;
            if (message.kind != null && Object.hasOwnProperty.call(message, "kind"))
                object.kind = message.kind;
            if (message.regime != null && Object.hasOwnProperty.call(message, "regime"))
                object.regime = message.regime;
            return object;
        };

        /**
         * Converts this DecodePoint to JSON.
         * @function toJSON
         * @memberof velobench.DecodePoint
         * @instance
         * @returns {Object.<string,*>} JSON object
         */
        DecodePoint.prototype.toJSON = function toJSON() {
            return this.constructor.toObject(this, $protobuf.util.toJSONOptions);
        };

        /**
         * Gets the default type url for DecodePoint
         * @function getTypeUrl
         * @memberof velobench.DecodePoint
         * @static
         * @param {string} [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns {string} The default type url
         */
        DecodePoint.getTypeUrl = function getTypeUrl(typeUrlPrefix) {
            if (typeUrlPrefix === undefined) {
                typeUrlPrefix = "type.googleapis.com";
            }
            return typeUrlPrefix + "/velobench.DecodePoint";
        };

        return DecodePoint;
    })();

    velobench.AcceptancePoint = (function() {

        /**
         * Properties of an AcceptancePoint.
         * @memberof velobench
         * @interface IAcceptancePoint
         * @property {number|null} [t] AcceptancePoint t
         * @property {number|null} [rate] AcceptancePoint rate
         */

        /**
         * Constructs a new AcceptancePoint.
         * @memberof velobench
         * @classdesc Represents an AcceptancePoint.
         * @implements IAcceptancePoint
         * @constructor
         * @param {velobench.IAcceptancePoint=} [properties] Properties to set
         */
        function AcceptancePoint(properties) {
            if (properties)
                for (let keys = Object.keys(properties), i = 0; i < keys.length; ++i)
                    if (properties[keys[i]] != null && keys[i] !== "__proto__")
                        this[keys[i]] = properties[keys[i]];
        }

        /**
         * AcceptancePoint t.
         * @member {number} t
         * @memberof velobench.AcceptancePoint
         * @instance
         */
        AcceptancePoint.prototype.t = 0;

        /**
         * AcceptancePoint rate.
         * @member {number} rate
         * @memberof velobench.AcceptancePoint
         * @instance
         */
        AcceptancePoint.prototype.rate = 0;

        /**
         * Creates a new AcceptancePoint instance using the specified properties.
         * @function create
         * @memberof velobench.AcceptancePoint
         * @static
         * @param {velobench.IAcceptancePoint=} [properties] Properties to set
         * @returns {velobench.AcceptancePoint} AcceptancePoint instance
         */
        AcceptancePoint.create = function create(properties) {
            return new AcceptancePoint(properties);
        };

        /**
         * Encodes the specified AcceptancePoint message. Does not implicitly {@link velobench.AcceptancePoint.verify|verify} messages.
         * @function encode
         * @memberof velobench.AcceptancePoint
         * @static
         * @param {velobench.IAcceptancePoint} message AcceptancePoint message or plain object to encode
         * @param {$protobuf.Writer} [writer] Writer to encode to
         * @returns {$protobuf.Writer} Writer
         */
        AcceptancePoint.encode = function encode(message, writer, q) {
            if (!writer)
                writer = $Writer.create();
            if (q === undefined)
                q = 0;
            if (q > $util.recursionLimit)
                throw Error("max depth exceeded");
            if (message.t != null && Object.hasOwnProperty.call(message, "t"))
                writer.uint32(/* id 1, wireType 1 =*/9).double(message.t);
            if (message.rate != null && Object.hasOwnProperty.call(message, "rate"))
                writer.uint32(/* id 2, wireType 1 =*/17).double(message.rate);
            return writer;
        };

        /**
         * Encodes the specified AcceptancePoint message, length delimited. Does not implicitly {@link velobench.AcceptancePoint.verify|verify} messages.
         * @function encodeDelimited
         * @memberof velobench.AcceptancePoint
         * @static
         * @param {velobench.IAcceptancePoint} message AcceptancePoint message or plain object to encode
         * @param {$protobuf.Writer} [writer] Writer to encode to
         * @returns {$protobuf.Writer} Writer
         */
        AcceptancePoint.encodeDelimited = function encodeDelimited(message, writer) {
            return this.encode(message, writer && writer.len ? writer.fork() : writer).ldelim();
        };

        /**
         * Decodes an AcceptancePoint message from the specified reader or buffer.
         * @function decode
         * @memberof velobench.AcceptancePoint
         * @static
         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
         * @param {number} [length] Message length if known beforehand
         * @returns {velobench.AcceptancePoint} AcceptancePoint
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        AcceptancePoint.decode = function decode(reader, length, error, long) {
            if (!(reader instanceof $Reader))
                reader = $Reader.create(reader);
            if (long === undefined)
                long = 0;
            if (long > $Reader.recursionLimit)
                throw Error("maximum nesting depth exceeded");
            let end, message;
            if (length === undefined)
                end = reader.len;
            else {
                end = reader.pos + length;
                if (end > reader.len)
                    throw RangeError("index out of range");
                length = reader.len;
                reader.len = end;
            }
            message = new $root.velobench.AcceptancePoint();
            while (reader.pos < end) {
                let tag = reader.uint32();
                if (tag === error)
                    break;
                switch (tag >>> 3) {
                case 1: {
                        message.t = reader.double();
                        break;
                    }
                case 2: {
                        message.rate = reader.double();
                        break;
                    }
                default:
                    reader.skipType(tag & 7, long);
                    break;
                }
            }
            if (length !== undefined) {
                if (reader.pos !== end)
                    throw RangeError("index out of range");
                reader.len = length;
            }
            return message;
        };

        /**
         * Decodes an AcceptancePoint message from the specified reader or buffer, length delimited.
         * @function decodeDelimited
         * @memberof velobench.AcceptancePoint
         * @static
         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
         * @returns {velobench.AcceptancePoint} AcceptancePoint
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        AcceptancePoint.decodeDelimited = function decodeDelimited(reader) {
            if (!(reader instanceof $Reader))
                reader = new $Reader(reader);
            return this.decode(reader, reader.uint32());
        };

        /**
         * Verifies an AcceptancePoint message.
         * @function verify
         * @memberof velobench.AcceptancePoint
         * @static
         * @param {Object.<string,*>} message Plain object to verify
         * @returns {string|null} `null` if valid, otherwise the reason why it is not
         */
        AcceptancePoint.verify = function verify(message, long) {
            if (typeof message !== "object" || message === null)
                return "object expected";
            if (long === undefined)
                long = 0;
            if (long > $util.recursionLimit)
                return "maximum nesting depth exceeded";
            if (message.t != null && Object.hasOwnProperty.call(message, "t"))
                if (typeof message.t !== "number")
                    return "t: number expected";
            if (message.rate != null && Object.hasOwnProperty.call(message, "rate"))
                if (typeof message.rate !== "number")
                    return "rate: number expected";
            return null;
        };

        /**
         * Creates an AcceptancePoint message from a plain object. Also converts values to their respective internal types.
         * @function fromObject
         * @memberof velobench.AcceptancePoint
         * @static
         * @param {Object.<string,*>} object Plain object
         * @returns {velobench.AcceptancePoint} AcceptancePoint
         */
        AcceptancePoint.fromObject = function fromObject(object, long) {
            if (object instanceof $root.velobench.AcceptancePoint)
                return object;
            if (!$util.isObject(object))
                throw TypeError(".velobench.AcceptancePoint: object expected");
            if (long === undefined)
                long = 0;
            if (long > $util.recursionLimit)
                throw Error("maximum nesting depth exceeded");
            let message = new $root.velobench.AcceptancePoint();
            if (object.t != null)
                message.t = Number(object.t);
            if (object.rate != null)
                message.rate = Number(object.rate);
            return message;
        };

        /**
         * Creates a plain object from an AcceptancePoint message. Also converts values to other types if specified.
         * @function toObject
         * @memberof velobench.AcceptancePoint
         * @static
         * @param {velobench.AcceptancePoint} message AcceptancePoint
         * @param {$protobuf.IConversionOptions} [options] Conversion options
         * @returns {Object.<string,*>} Plain object
         */
        AcceptancePoint.toObject = function toObject(message, options, q) {
            if (!options)
                options = {};
            if (q === undefined)
                q = 0;
            if (q > $util.recursionLimit)
                throw Error("max depth exceeded");
            let object = {};
            if (options.defaults) {
                object.t = 0;
                object.rate = 0;
            }
            if (message.t != null && Object.hasOwnProperty.call(message, "t"))
                object.t = options.json && !isFinite(message.t) ? String(message.t) : message.t;
            if (message.rate != null && Object.hasOwnProperty.call(message, "rate"))
                object.rate = options.json && !isFinite(message.rate) ? String(message.rate) : message.rate;
            return object;
        };

        /**
         * Converts this AcceptancePoint to JSON.
         * @function toJSON
         * @memberof velobench.AcceptancePoint
         * @instance
         * @returns {Object.<string,*>} JSON object
         */
        AcceptancePoint.prototype.toJSON = function toJSON() {
            return this.constructor.toObject(this, $protobuf.util.toJSONOptions);
        };

        /**
         * Gets the default type url for AcceptancePoint
         * @function getTypeUrl
         * @memberof velobench.AcceptancePoint
         * @static
         * @param {string} [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns {string} The default type url
         */
        AcceptancePoint.getTypeUrl = function getTypeUrl(typeUrlPrefix) {
            if (typeUrlPrefix === undefined) {
                typeUrlPrefix = "type.googleapis.com";
            }
            return typeUrlPrefix + "/velobench.AcceptancePoint";
        };

        return AcceptancePoint;
    })();

    velobench.SpecDepthPoint = (function() {

        /**
         * Properties of a SpecDepthPoint.
         * @memberof velobench
         * @interface ISpecDepthPoint
         * @property {number|null} [depth] SpecDepthPoint depth
         * @property {number|null} [count] SpecDepthPoint count
         */

        /**
         * Constructs a new SpecDepthPoint.
         * @memberof velobench
         * @classdesc Represents a SpecDepthPoint.
         * @implements ISpecDepthPoint
         * @constructor
         * @param {velobench.ISpecDepthPoint=} [properties] Properties to set
         */
        function SpecDepthPoint(properties) {
            if (properties)
                for (let keys = Object.keys(properties), i = 0; i < keys.length; ++i)
                    if (properties[keys[i]] != null && keys[i] !== "__proto__")
                        this[keys[i]] = properties[keys[i]];
        }

        /**
         * SpecDepthPoint depth.
         * @member {number} depth
         * @memberof velobench.SpecDepthPoint
         * @instance
         */
        SpecDepthPoint.prototype.depth = 0;

        /**
         * SpecDepthPoint count.
         * @member {number} count
         * @memberof velobench.SpecDepthPoint
         * @instance
         */
        SpecDepthPoint.prototype.count = 0;

        /**
         * Creates a new SpecDepthPoint instance using the specified properties.
         * @function create
         * @memberof velobench.SpecDepthPoint
         * @static
         * @param {velobench.ISpecDepthPoint=} [properties] Properties to set
         * @returns {velobench.SpecDepthPoint} SpecDepthPoint instance
         */
        SpecDepthPoint.create = function create(properties) {
            return new SpecDepthPoint(properties);
        };

        /**
         * Encodes the specified SpecDepthPoint message. Does not implicitly {@link velobench.SpecDepthPoint.verify|verify} messages.
         * @function encode
         * @memberof velobench.SpecDepthPoint
         * @static
         * @param {velobench.ISpecDepthPoint} message SpecDepthPoint message or plain object to encode
         * @param {$protobuf.Writer} [writer] Writer to encode to
         * @returns {$protobuf.Writer} Writer
         */
        SpecDepthPoint.encode = function encode(message, writer, q) {
            if (!writer)
                writer = $Writer.create();
            if (q === undefined)
                q = 0;
            if (q > $util.recursionLimit)
                throw Error("max depth exceeded");
            if (message.depth != null && Object.hasOwnProperty.call(message, "depth"))
                writer.uint32(/* id 1, wireType 0 =*/8).uint32(message.depth);
            if (message.count != null && Object.hasOwnProperty.call(message, "count"))
                writer.uint32(/* id 2, wireType 0 =*/16).uint32(message.count);
            return writer;
        };

        /**
         * Encodes the specified SpecDepthPoint message, length delimited. Does not implicitly {@link velobench.SpecDepthPoint.verify|verify} messages.
         * @function encodeDelimited
         * @memberof velobench.SpecDepthPoint
         * @static
         * @param {velobench.ISpecDepthPoint} message SpecDepthPoint message or plain object to encode
         * @param {$protobuf.Writer} [writer] Writer to encode to
         * @returns {$protobuf.Writer} Writer
         */
        SpecDepthPoint.encodeDelimited = function encodeDelimited(message, writer) {
            return this.encode(message, writer && writer.len ? writer.fork() : writer).ldelim();
        };

        /**
         * Decodes a SpecDepthPoint message from the specified reader or buffer.
         * @function decode
         * @memberof velobench.SpecDepthPoint
         * @static
         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
         * @param {number} [length] Message length if known beforehand
         * @returns {velobench.SpecDepthPoint} SpecDepthPoint
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        SpecDepthPoint.decode = function decode(reader, length, error, long) {
            if (!(reader instanceof $Reader))
                reader = $Reader.create(reader);
            if (long === undefined)
                long = 0;
            if (long > $Reader.recursionLimit)
                throw Error("maximum nesting depth exceeded");
            let end, message;
            if (length === undefined)
                end = reader.len;
            else {
                end = reader.pos + length;
                if (end > reader.len)
                    throw RangeError("index out of range");
                length = reader.len;
                reader.len = end;
            }
            message = new $root.velobench.SpecDepthPoint();
            while (reader.pos < end) {
                let tag = reader.uint32();
                if (tag === error)
                    break;
                switch (tag >>> 3) {
                case 1: {
                        message.depth = reader.uint32();
                        break;
                    }
                case 2: {
                        message.count = reader.uint32();
                        break;
                    }
                default:
                    reader.skipType(tag & 7, long);
                    break;
                }
            }
            if (length !== undefined) {
                if (reader.pos !== end)
                    throw RangeError("index out of range");
                reader.len = length;
            }
            return message;
        };

        /**
         * Decodes a SpecDepthPoint message from the specified reader or buffer, length delimited.
         * @function decodeDelimited
         * @memberof velobench.SpecDepthPoint
         * @static
         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
         * @returns {velobench.SpecDepthPoint} SpecDepthPoint
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        SpecDepthPoint.decodeDelimited = function decodeDelimited(reader) {
            if (!(reader instanceof $Reader))
                reader = new $Reader(reader);
            return this.decode(reader, reader.uint32());
        };

        /**
         * Verifies a SpecDepthPoint message.
         * @function verify
         * @memberof velobench.SpecDepthPoint
         * @static
         * @param {Object.<string,*>} message Plain object to verify
         * @returns {string|null} `null` if valid, otherwise the reason why it is not
         */
        SpecDepthPoint.verify = function verify(message, long) {
            if (typeof message !== "object" || message === null)
                return "object expected";
            if (long === undefined)
                long = 0;
            if (long > $util.recursionLimit)
                return "maximum nesting depth exceeded";
            if (message.depth != null && Object.hasOwnProperty.call(message, "depth"))
                if (!$util.isInteger(message.depth))
                    return "depth: integer expected";
            if (message.count != null && Object.hasOwnProperty.call(message, "count"))
                if (!$util.isInteger(message.count))
                    return "count: integer expected";
            return null;
        };

        /**
         * Creates a SpecDepthPoint message from a plain object. Also converts values to their respective internal types.
         * @function fromObject
         * @memberof velobench.SpecDepthPoint
         * @static
         * @param {Object.<string,*>} object Plain object
         * @returns {velobench.SpecDepthPoint} SpecDepthPoint
         */
        SpecDepthPoint.fromObject = function fromObject(object, long) {
            if (object instanceof $root.velobench.SpecDepthPoint)
                return object;
            if (!$util.isObject(object))
                throw TypeError(".velobench.SpecDepthPoint: object expected");
            if (long === undefined)
                long = 0;
            if (long > $util.recursionLimit)
                throw Error("maximum nesting depth exceeded");
            let message = new $root.velobench.SpecDepthPoint();
            if (object.depth != null)
                message.depth = object.depth >>> 0;
            if (object.count != null)
                message.count = object.count >>> 0;
            return message;
        };

        /**
         * Creates a plain object from a SpecDepthPoint message. Also converts values to other types if specified.
         * @function toObject
         * @memberof velobench.SpecDepthPoint
         * @static
         * @param {velobench.SpecDepthPoint} message SpecDepthPoint
         * @param {$protobuf.IConversionOptions} [options] Conversion options
         * @returns {Object.<string,*>} Plain object
         */
        SpecDepthPoint.toObject = function toObject(message, options, q) {
            if (!options)
                options = {};
            if (q === undefined)
                q = 0;
            if (q > $util.recursionLimit)
                throw Error("max depth exceeded");
            let object = {};
            if (options.defaults) {
                object.depth = 0;
                object.count = 0;
            }
            if (message.depth != null && Object.hasOwnProperty.call(message, "depth"))
                object.depth = message.depth;
            if (message.count != null && Object.hasOwnProperty.call(message, "count"))
                object.count = message.count;
            return object;
        };

        /**
         * Converts this SpecDepthPoint to JSON.
         * @function toJSON
         * @memberof velobench.SpecDepthPoint
         * @instance
         * @returns {Object.<string,*>} JSON object
         */
        SpecDepthPoint.prototype.toJSON = function toJSON() {
            return this.constructor.toObject(this, $protobuf.util.toJSONOptions);
        };

        /**
         * Gets the default type url for SpecDepthPoint
         * @function getTypeUrl
         * @memberof velobench.SpecDepthPoint
         * @static
         * @param {string} [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns {string} The default type url
         */
        SpecDepthPoint.getTypeUrl = function getTypeUrl(typeUrlPrefix) {
            if (typeUrlPrefix === undefined) {
                typeUrlPrefix = "type.googleapis.com";
            }
            return typeUrlPrefix + "/velobench.SpecDepthPoint";
        };

        return SpecDepthPoint;
    })();

    velobench.Cluster = (function() {

        /**
         * Properties of a Cluster.
         * @memberof velobench
         * @interface ICluster
         * @property {number|null} [mean] Cluster mean
         * @property {number|null} [count] Cluster count
         * @property {number|null} [std] Cluster std
         * @property {number|null} [min] Cluster min
         * @property {number|null} [max] Cluster max
         */

        /**
         * Constructs a new Cluster.
         * @memberof velobench
         * @classdesc Represents a Cluster.
         * @implements ICluster
         * @constructor
         * @param {velobench.ICluster=} [properties] Properties to set
         */
        function Cluster(properties) {
            if (properties)
                for (let keys = Object.keys(properties), i = 0; i < keys.length; ++i)
                    if (properties[keys[i]] != null && keys[i] !== "__proto__")
                        this[keys[i]] = properties[keys[i]];
        }

        /**
         * Cluster mean.
         * @member {number} mean
         * @memberof velobench.Cluster
         * @instance
         */
        Cluster.prototype.mean = 0;

        /**
         * Cluster count.
         * @member {number} count
         * @memberof velobench.Cluster
         * @instance
         */
        Cluster.prototype.count = 0;

        /**
         * Cluster std.
         * @member {number} std
         * @memberof velobench.Cluster
         * @instance
         */
        Cluster.prototype.std = 0;

        /**
         * Cluster min.
         * @member {number} min
         * @memberof velobench.Cluster
         * @instance
         */
        Cluster.prototype.min = 0;

        /**
         * Cluster max.
         * @member {number} max
         * @memberof velobench.Cluster
         * @instance
         */
        Cluster.prototype.max = 0;

        /**
         * Creates a new Cluster instance using the specified properties.
         * @function create
         * @memberof velobench.Cluster
         * @static
         * @param {velobench.ICluster=} [properties] Properties to set
         * @returns {velobench.Cluster} Cluster instance
         */
        Cluster.create = function create(properties) {
            return new Cluster(properties);
        };

        /**
         * Encodes the specified Cluster message. Does not implicitly {@link velobench.Cluster.verify|verify} messages.
         * @function encode
         * @memberof velobench.Cluster
         * @static
         * @param {velobench.ICluster} message Cluster message or plain object to encode
         * @param {$protobuf.Writer} [writer] Writer to encode to
         * @returns {$protobuf.Writer} Writer
         */
        Cluster.encode = function encode(message, writer, q) {
            if (!writer)
                writer = $Writer.create();
            if (q === undefined)
                q = 0;
            if (q > $util.recursionLimit)
                throw Error("max depth exceeded");
            if (message.mean != null && Object.hasOwnProperty.call(message, "mean"))
                writer.uint32(/* id 1, wireType 1 =*/9).double(message.mean);
            if (message.count != null && Object.hasOwnProperty.call(message, "count"))
                writer.uint32(/* id 2, wireType 0 =*/16).uint32(message.count);
            if (message.std != null && Object.hasOwnProperty.call(message, "std"))
                writer.uint32(/* id 3, wireType 1 =*/25).double(message.std);
            if (message.min != null && Object.hasOwnProperty.call(message, "min"))
                writer.uint32(/* id 4, wireType 1 =*/33).double(message.min);
            if (message.max != null && Object.hasOwnProperty.call(message, "max"))
                writer.uint32(/* id 5, wireType 1 =*/41).double(message.max);
            return writer;
        };

        /**
         * Encodes the specified Cluster message, length delimited. Does not implicitly {@link velobench.Cluster.verify|verify} messages.
         * @function encodeDelimited
         * @memberof velobench.Cluster
         * @static
         * @param {velobench.ICluster} message Cluster message or plain object to encode
         * @param {$protobuf.Writer} [writer] Writer to encode to
         * @returns {$protobuf.Writer} Writer
         */
        Cluster.encodeDelimited = function encodeDelimited(message, writer) {
            return this.encode(message, writer && writer.len ? writer.fork() : writer).ldelim();
        };

        /**
         * Decodes a Cluster message from the specified reader or buffer.
         * @function decode
         * @memberof velobench.Cluster
         * @static
         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
         * @param {number} [length] Message length if known beforehand
         * @returns {velobench.Cluster} Cluster
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        Cluster.decode = function decode(reader, length, error, long) {
            if (!(reader instanceof $Reader))
                reader = $Reader.create(reader);
            if (long === undefined)
                long = 0;
            if (long > $Reader.recursionLimit)
                throw Error("maximum nesting depth exceeded");
            let end, message;
            if (length === undefined)
                end = reader.len;
            else {
                end = reader.pos + length;
                if (end > reader.len)
                    throw RangeError("index out of range");
                length = reader.len;
                reader.len = end;
            }
            message = new $root.velobench.Cluster();
            while (reader.pos < end) {
                let tag = reader.uint32();
                if (tag === error)
                    break;
                switch (tag >>> 3) {
                case 1: {
                        message.mean = reader.double();
                        break;
                    }
                case 2: {
                        message.count = reader.uint32();
                        break;
                    }
                case 3: {
                        message.std = reader.double();
                        break;
                    }
                case 4: {
                        message.min = reader.double();
                        break;
                    }
                case 5: {
                        message.max = reader.double();
                        break;
                    }
                default:
                    reader.skipType(tag & 7, long);
                    break;
                }
            }
            if (length !== undefined) {
                if (reader.pos !== end)
                    throw RangeError("index out of range");
                reader.len = length;
            }
            return message;
        };

        /**
         * Decodes a Cluster message from the specified reader or buffer, length delimited.
         * @function decodeDelimited
         * @memberof velobench.Cluster
         * @static
         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
         * @returns {velobench.Cluster} Cluster
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        Cluster.decodeDelimited = function decodeDelimited(reader) {
            if (!(reader instanceof $Reader))
                reader = new $Reader(reader);
            return this.decode(reader, reader.uint32());
        };

        /**
         * Verifies a Cluster message.
         * @function verify
         * @memberof velobench.Cluster
         * @static
         * @param {Object.<string,*>} message Plain object to verify
         * @returns {string|null} `null` if valid, otherwise the reason why it is not
         */
        Cluster.verify = function verify(message, long) {
            if (typeof message !== "object" || message === null)
                return "object expected";
            if (long === undefined)
                long = 0;
            if (long > $util.recursionLimit)
                return "maximum nesting depth exceeded";
            if (message.mean != null && Object.hasOwnProperty.call(message, "mean"))
                if (typeof message.mean !== "number")
                    return "mean: number expected";
            if (message.count != null && Object.hasOwnProperty.call(message, "count"))
                if (!$util.isInteger(message.count))
                    return "count: integer expected";
            if (message.std != null && Object.hasOwnProperty.call(message, "std"))
                if (typeof message.std !== "number")
                    return "std: number expected";
            if (message.min != null && Object.hasOwnProperty.call(message, "min"))
                if (typeof message.min !== "number")
                    return "min: number expected";
            if (message.max != null && Object.hasOwnProperty.call(message, "max"))
                if (typeof message.max !== "number")
                    return "max: number expected";
            return null;
        };

        /**
         * Creates a Cluster message from a plain object. Also converts values to their respective internal types.
         * @function fromObject
         * @memberof velobench.Cluster
         * @static
         * @param {Object.<string,*>} object Plain object
         * @returns {velobench.Cluster} Cluster
         */
        Cluster.fromObject = function fromObject(object, long) {
            if (object instanceof $root.velobench.Cluster)
                return object;
            if (!$util.isObject(object))
                throw TypeError(".velobench.Cluster: object expected");
            if (long === undefined)
                long = 0;
            if (long > $util.recursionLimit)
                throw Error("maximum nesting depth exceeded");
            let message = new $root.velobench.Cluster();
            if (object.mean != null)
                message.mean = Number(object.mean);
            if (object.count != null)
                message.count = object.count >>> 0;
            if (object.std != null)
                message.std = Number(object.std);
            if (object.min != null)
                message.min = Number(object.min);
            if (object.max != null)
                message.max = Number(object.max);
            return message;
        };

        /**
         * Creates a plain object from a Cluster message. Also converts values to other types if specified.
         * @function toObject
         * @memberof velobench.Cluster
         * @static
         * @param {velobench.Cluster} message Cluster
         * @param {$protobuf.IConversionOptions} [options] Conversion options
         * @returns {Object.<string,*>} Plain object
         */
        Cluster.toObject = function toObject(message, options, q) {
            if (!options)
                options = {};
            if (q === undefined)
                q = 0;
            if (q > $util.recursionLimit)
                throw Error("max depth exceeded");
            let object = {};
            if (options.defaults) {
                object.mean = 0;
                object.count = 0;
                object.std = 0;
                object.min = 0;
                object.max = 0;
            }
            if (message.mean != null && Object.hasOwnProperty.call(message, "mean"))
                object.mean = options.json && !isFinite(message.mean) ? String(message.mean) : message.mean;
            if (message.count != null && Object.hasOwnProperty.call(message, "count"))
                object.count = message.count;
            if (message.std != null && Object.hasOwnProperty.call(message, "std"))
                object.std = options.json && !isFinite(message.std) ? String(message.std) : message.std;
            if (message.min != null && Object.hasOwnProperty.call(message, "min"))
                object.min = options.json && !isFinite(message.min) ? String(message.min) : message.min;
            if (message.max != null && Object.hasOwnProperty.call(message, "max"))
                object.max = options.json && !isFinite(message.max) ? String(message.max) : message.max;
            return object;
        };

        /**
         * Converts this Cluster to JSON.
         * @function toJSON
         * @memberof velobench.Cluster
         * @instance
         * @returns {Object.<string,*>} JSON object
         */
        Cluster.prototype.toJSON = function toJSON() {
            return this.constructor.toObject(this, $protobuf.util.toJSONOptions);
        };

        /**
         * Gets the default type url for Cluster
         * @function getTypeUrl
         * @memberof velobench.Cluster
         * @static
         * @param {string} [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns {string} The default type url
         */
        Cluster.getTypeUrl = function getTypeUrl(typeUrlPrefix) {
            if (typeUrlPrefix === undefined) {
                typeUrlPrefix = "type.googleapis.com";
            }
            return typeUrlPrefix + "/velobench.Cluster";
        };

        return Cluster;
    })();

    velobench.ClusterResult = (function() {

        /**
         * Properties of a ClusterResult.
         * @memberof velobench
         * @interface IClusterResult
         * @property {boolean|null} [bimodal] ClusterResult bimodal
         * @property {number|null} [split] ClusterResult split
         * @property {number|null} [eta] ClusterResult eta
         * @property {Array.<velobench.ICluster>|null} [clusters] ClusterResult clusters
         * @property {number|null} [total] ClusterResult total
         */

        /**
         * Constructs a new ClusterResult.
         * @memberof velobench
         * @classdesc Represents a ClusterResult.
         * @implements IClusterResult
         * @constructor
         * @param {velobench.IClusterResult=} [properties] Properties to set
         */
        function ClusterResult(properties) {
            this.clusters = [];
            if (properties)
                for (let keys = Object.keys(properties), i = 0; i < keys.length; ++i)
                    if (properties[keys[i]] != null && keys[i] !== "__proto__")
                        this[keys[i]] = properties[keys[i]];
        }

        /**
         * ClusterResult bimodal.
         * @member {boolean} bimodal
         * @memberof velobench.ClusterResult
         * @instance
         */
        ClusterResult.prototype.bimodal = false;

        /**
         * ClusterResult split.
         * @member {number} split
         * @memberof velobench.ClusterResult
         * @instance
         */
        ClusterResult.prototype.split = 0;

        /**
         * ClusterResult eta.
         * @member {number} eta
         * @memberof velobench.ClusterResult
         * @instance
         */
        ClusterResult.prototype.eta = 0;

        /**
         * ClusterResult clusters.
         * @member {Array.<velobench.ICluster>} clusters
         * @memberof velobench.ClusterResult
         * @instance
         */
        ClusterResult.prototype.clusters = $util.emptyArray;

        /**
         * ClusterResult total.
         * @member {number} total
         * @memberof velobench.ClusterResult
         * @instance
         */
        ClusterResult.prototype.total = 0;

        /**
         * Creates a new ClusterResult instance using the specified properties.
         * @function create
         * @memberof velobench.ClusterResult
         * @static
         * @param {velobench.IClusterResult=} [properties] Properties to set
         * @returns {velobench.ClusterResult} ClusterResult instance
         */
        ClusterResult.create = function create(properties) {
            return new ClusterResult(properties);
        };

        /**
         * Encodes the specified ClusterResult message. Does not implicitly {@link velobench.ClusterResult.verify|verify} messages.
         * @function encode
         * @memberof velobench.ClusterResult
         * @static
         * @param {velobench.IClusterResult} message ClusterResult message or plain object to encode
         * @param {$protobuf.Writer} [writer] Writer to encode to
         * @returns {$protobuf.Writer} Writer
         */
        ClusterResult.encode = function encode(message, writer, q) {
            if (!writer)
                writer = $Writer.create();
            if (q === undefined)
                q = 0;
            if (q > $util.recursionLimit)
                throw Error("max depth exceeded");
            if (message.bimodal != null && Object.hasOwnProperty.call(message, "bimodal"))
                writer.uint32(/* id 1, wireType 0 =*/8).bool(message.bimodal);
            if (message.split != null && Object.hasOwnProperty.call(message, "split"))
                writer.uint32(/* id 2, wireType 1 =*/17).double(message.split);
            if (message.eta != null && Object.hasOwnProperty.call(message, "eta"))
                writer.uint32(/* id 3, wireType 1 =*/25).double(message.eta);
            if (message.clusters != null && message.clusters.length)
                for (let i = 0; i < message.clusters.length; ++i)
                    $root.velobench.Cluster.encode(message.clusters[i], writer.uint32(/* id 4, wireType 2 =*/34).fork(), q + 1).ldelim();
            if (message.total != null && Object.hasOwnProperty.call(message, "total"))
                writer.uint32(/* id 5, wireType 0 =*/40).uint32(message.total);
            return writer;
        };

        /**
         * Encodes the specified ClusterResult message, length delimited. Does not implicitly {@link velobench.ClusterResult.verify|verify} messages.
         * @function encodeDelimited
         * @memberof velobench.ClusterResult
         * @static
         * @param {velobench.IClusterResult} message ClusterResult message or plain object to encode
         * @param {$protobuf.Writer} [writer] Writer to encode to
         * @returns {$protobuf.Writer} Writer
         */
        ClusterResult.encodeDelimited = function encodeDelimited(message, writer) {
            return this.encode(message, writer && writer.len ? writer.fork() : writer).ldelim();
        };

        /**
         * Decodes a ClusterResult message from the specified reader or buffer.
         * @function decode
         * @memberof velobench.ClusterResult
         * @static
         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
         * @param {number} [length] Message length if known beforehand
         * @returns {velobench.ClusterResult} ClusterResult
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        ClusterResult.decode = function decode(reader, length, error, long) {
            if (!(reader instanceof $Reader))
                reader = $Reader.create(reader);
            if (long === undefined)
                long = 0;
            if (long > $Reader.recursionLimit)
                throw Error("maximum nesting depth exceeded");
            let end, message;
            if (length === undefined)
                end = reader.len;
            else {
                end = reader.pos + length;
                if (end > reader.len)
                    throw RangeError("index out of range");
                length = reader.len;
                reader.len = end;
            }
            message = new $root.velobench.ClusterResult();
            while (reader.pos < end) {
                let tag = reader.uint32();
                if (tag === error)
                    break;
                switch (tag >>> 3) {
                case 1: {
                        message.bimodal = reader.bool();
                        break;
                    }
                case 2: {
                        message.split = reader.double();
                        break;
                    }
                case 3: {
                        message.eta = reader.double();
                        break;
                    }
                case 4: {
                        if (!(message.clusters && message.clusters.length))
                            message.clusters = [];
                        message.clusters.push($root.velobench.Cluster.decode(reader, reader.uint32(), undefined, long + 1));
                        break;
                    }
                case 5: {
                        message.total = reader.uint32();
                        break;
                    }
                default:
                    reader.skipType(tag & 7, long);
                    break;
                }
            }
            if (length !== undefined) {
                if (reader.pos !== end)
                    throw RangeError("index out of range");
                reader.len = length;
            }
            return message;
        };

        /**
         * Decodes a ClusterResult message from the specified reader or buffer, length delimited.
         * @function decodeDelimited
         * @memberof velobench.ClusterResult
         * @static
         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
         * @returns {velobench.ClusterResult} ClusterResult
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        ClusterResult.decodeDelimited = function decodeDelimited(reader) {
            if (!(reader instanceof $Reader))
                reader = new $Reader(reader);
            return this.decode(reader, reader.uint32());
        };

        /**
         * Verifies a ClusterResult message.
         * @function verify
         * @memberof velobench.ClusterResult
         * @static
         * @param {Object.<string,*>} message Plain object to verify
         * @returns {string|null} `null` if valid, otherwise the reason why it is not
         */
        ClusterResult.verify = function verify(message, long) {
            if (typeof message !== "object" || message === null)
                return "object expected";
            if (long === undefined)
                long = 0;
            if (long > $util.recursionLimit)
                return "maximum nesting depth exceeded";
            if (message.bimodal != null && Object.hasOwnProperty.call(message, "bimodal"))
                if (typeof message.bimodal !== "boolean")
                    return "bimodal: boolean expected";
            if (message.split != null && Object.hasOwnProperty.call(message, "split"))
                if (typeof message.split !== "number")
                    return "split: number expected";
            if (message.eta != null && Object.hasOwnProperty.call(message, "eta"))
                if (typeof message.eta !== "number")
                    return "eta: number expected";
            if (message.clusters != null && Object.hasOwnProperty.call(message, "clusters")) {
                if (!Array.isArray(message.clusters))
                    return "clusters: array expected";
                for (let i = 0; i < message.clusters.length; ++i) {
                    let error = $root.velobench.Cluster.verify(message.clusters[i], long + 1);
                    if (error)
                        return "clusters." + error;
                }
            }
            if (message.total != null && Object.hasOwnProperty.call(message, "total"))
                if (!$util.isInteger(message.total))
                    return "total: integer expected";
            return null;
        };

        /**
         * Creates a ClusterResult message from a plain object. Also converts values to their respective internal types.
         * @function fromObject
         * @memberof velobench.ClusterResult
         * @static
         * @param {Object.<string,*>} object Plain object
         * @returns {velobench.ClusterResult} ClusterResult
         */
        ClusterResult.fromObject = function fromObject(object, long) {
            if (object instanceof $root.velobench.ClusterResult)
                return object;
            if (!$util.isObject(object))
                throw TypeError(".velobench.ClusterResult: object expected");
            if (long === undefined)
                long = 0;
            if (long > $util.recursionLimit)
                throw Error("maximum nesting depth exceeded");
            let message = new $root.velobench.ClusterResult();
            if (object.bimodal != null)
                message.bimodal = Boolean(object.bimodal);
            if (object.split != null)
                message.split = Number(object.split);
            if (object.eta != null)
                message.eta = Number(object.eta);
            if (object.clusters) {
                if (!Array.isArray(object.clusters))
                    throw TypeError(".velobench.ClusterResult.clusters: array expected");
                message.clusters = [];
                for (let i = 0; i < object.clusters.length; ++i) {
                    if (!$util.isObject(object.clusters[i]))
                        throw TypeError(".velobench.ClusterResult.clusters: object expected");
                    message.clusters[i] = $root.velobench.Cluster.fromObject(object.clusters[i], long + 1);
                }
            }
            if (object.total != null)
                message.total = object.total >>> 0;
            return message;
        };

        /**
         * Creates a plain object from a ClusterResult message. Also converts values to other types if specified.
         * @function toObject
         * @memberof velobench.ClusterResult
         * @static
         * @param {velobench.ClusterResult} message ClusterResult
         * @param {$protobuf.IConversionOptions} [options] Conversion options
         * @returns {Object.<string,*>} Plain object
         */
        ClusterResult.toObject = function toObject(message, options, q) {
            if (!options)
                options = {};
            if (q === undefined)
                q = 0;
            if (q > $util.recursionLimit)
                throw Error("max depth exceeded");
            let object = {};
            if (options.arrays || options.defaults)
                object.clusters = [];
            if (options.defaults) {
                object.bimodal = false;
                object.split = 0;
                object.eta = 0;
                object.total = 0;
            }
            if (message.bimodal != null && Object.hasOwnProperty.call(message, "bimodal"))
                object.bimodal = message.bimodal;
            if (message.split != null && Object.hasOwnProperty.call(message, "split"))
                object.split = options.json && !isFinite(message.split) ? String(message.split) : message.split;
            if (message.eta != null && Object.hasOwnProperty.call(message, "eta"))
                object.eta = options.json && !isFinite(message.eta) ? String(message.eta) : message.eta;
            if (message.clusters && message.clusters.length) {
                object.clusters = [];
                for (let j = 0; j < message.clusters.length; ++j)
                    object.clusters[j] = $root.velobench.Cluster.toObject(message.clusters[j], options, q + 1);
            }
            if (message.total != null && Object.hasOwnProperty.call(message, "total"))
                object.total = message.total;
            return object;
        };

        /**
         * Converts this ClusterResult to JSON.
         * @function toJSON
         * @memberof velobench.ClusterResult
         * @instance
         * @returns {Object.<string,*>} JSON object
         */
        ClusterResult.prototype.toJSON = function toJSON() {
            return this.constructor.toObject(this, $protobuf.util.toJSONOptions);
        };

        /**
         * Gets the default type url for ClusterResult
         * @function getTypeUrl
         * @memberof velobench.ClusterResult
         * @static
         * @param {string} [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns {string} The default type url
         */
        ClusterResult.getTypeUrl = function getTypeUrl(typeUrlPrefix) {
            if (typeUrlPrefix === undefined) {
                typeUrlPrefix = "type.googleapis.com";
            }
            return typeUrlPrefix + "/velobench.ClusterResult";
        };

        return ClusterResult;
    })();

    velobench.LiveStat = (function() {

        /**
         * Properties of a LiveStat.
         * @memberof velobench
         * @interface ILiveStat
         * @property {number|null} [tokS] LiveStat tokS
         * @property {number|null} [avg] LiveStat avg
         * @property {number|null} [min] LiveStat min
         * @property {number|null} [median] LiveStat median
         * @property {number|null} [max] LiveStat max
         * @property {number|null} [tokens] LiveStat tokens
         * @property {number|null} [ttftMs] LiveStat ttftMs
         * @property {number|null} [genMs] LiveStat genMs
         * @property {number|null} [reasoningTokens] LiveStat reasoningTokens
         * @property {number|null} [contentTokens] LiveStat contentTokens
         */

        /**
         * Constructs a new LiveStat.
         * @memberof velobench
         * @classdesc Represents a LiveStat.
         * @implements ILiveStat
         * @constructor
         * @param {velobench.ILiveStat=} [properties] Properties to set
         */
        function LiveStat(properties) {
            if (properties)
                for (let keys = Object.keys(properties), i = 0; i < keys.length; ++i)
                    if (properties[keys[i]] != null && keys[i] !== "__proto__")
                        this[keys[i]] = properties[keys[i]];
        }

        /**
         * LiveStat tokS.
         * @member {number} tokS
         * @memberof velobench.LiveStat
         * @instance
         */
        LiveStat.prototype.tokS = 0;

        /**
         * LiveStat avg.
         * @member {number} avg
         * @memberof velobench.LiveStat
         * @instance
         */
        LiveStat.prototype.avg = 0;

        /**
         * LiveStat min.
         * @member {number} min
         * @memberof velobench.LiveStat
         * @instance
         */
        LiveStat.prototype.min = 0;

        /**
         * LiveStat median.
         * @member {number} median
         * @memberof velobench.LiveStat
         * @instance
         */
        LiveStat.prototype.median = 0;

        /**
         * LiveStat max.
         * @member {number} max
         * @memberof velobench.LiveStat
         * @instance
         */
        LiveStat.prototype.max = 0;

        /**
         * LiveStat tokens.
         * @member {number} tokens
         * @memberof velobench.LiveStat
         * @instance
         */
        LiveStat.prototype.tokens = 0;

        /**
         * LiveStat ttftMs.
         * @member {number} ttftMs
         * @memberof velobench.LiveStat
         * @instance
         */
        LiveStat.prototype.ttftMs = 0;

        /**
         * LiveStat genMs.
         * @member {number} genMs
         * @memberof velobench.LiveStat
         * @instance
         */
        LiveStat.prototype.genMs = 0;

        /**
         * LiveStat reasoningTokens.
         * @member {number} reasoningTokens
         * @memberof velobench.LiveStat
         * @instance
         */
        LiveStat.prototype.reasoningTokens = 0;

        /**
         * LiveStat contentTokens.
         * @member {number} contentTokens
         * @memberof velobench.LiveStat
         * @instance
         */
        LiveStat.prototype.contentTokens = 0;

        /**
         * Creates a new LiveStat instance using the specified properties.
         * @function create
         * @memberof velobench.LiveStat
         * @static
         * @param {velobench.ILiveStat=} [properties] Properties to set
         * @returns {velobench.LiveStat} LiveStat instance
         */
        LiveStat.create = function create(properties) {
            return new LiveStat(properties);
        };

        /**
         * Encodes the specified LiveStat message. Does not implicitly {@link velobench.LiveStat.verify|verify} messages.
         * @function encode
         * @memberof velobench.LiveStat
         * @static
         * @param {velobench.ILiveStat} message LiveStat message or plain object to encode
         * @param {$protobuf.Writer} [writer] Writer to encode to
         * @returns {$protobuf.Writer} Writer
         */
        LiveStat.encode = function encode(message, writer, q) {
            if (!writer)
                writer = $Writer.create();
            if (q === undefined)
                q = 0;
            if (q > $util.recursionLimit)
                throw Error("max depth exceeded");
            if (message.tokS != null && Object.hasOwnProperty.call(message, "tokS"))
                writer.uint32(/* id 1, wireType 1 =*/9).double(message.tokS);
            if (message.avg != null && Object.hasOwnProperty.call(message, "avg"))
                writer.uint32(/* id 2, wireType 1 =*/17).double(message.avg);
            if (message.min != null && Object.hasOwnProperty.call(message, "min"))
                writer.uint32(/* id 3, wireType 1 =*/25).double(message.min);
            if (message.median != null && Object.hasOwnProperty.call(message, "median"))
                writer.uint32(/* id 4, wireType 1 =*/33).double(message.median);
            if (message.max != null && Object.hasOwnProperty.call(message, "max"))
                writer.uint32(/* id 5, wireType 1 =*/41).double(message.max);
            if (message.tokens != null && Object.hasOwnProperty.call(message, "tokens"))
                writer.uint32(/* id 6, wireType 1 =*/49).double(message.tokens);
            if (message.ttftMs != null && Object.hasOwnProperty.call(message, "ttftMs"))
                writer.uint32(/* id 7, wireType 1 =*/57).double(message.ttftMs);
            if (message.genMs != null && Object.hasOwnProperty.call(message, "genMs"))
                writer.uint32(/* id 8, wireType 1 =*/65).double(message.genMs);
            if (message.reasoningTokens != null && Object.hasOwnProperty.call(message, "reasoningTokens"))
                writer.uint32(/* id 9, wireType 1 =*/73).double(message.reasoningTokens);
            if (message.contentTokens != null && Object.hasOwnProperty.call(message, "contentTokens"))
                writer.uint32(/* id 10, wireType 1 =*/81).double(message.contentTokens);
            return writer;
        };

        /**
         * Encodes the specified LiveStat message, length delimited. Does not implicitly {@link velobench.LiveStat.verify|verify} messages.
         * @function encodeDelimited
         * @memberof velobench.LiveStat
         * @static
         * @param {velobench.ILiveStat} message LiveStat message or plain object to encode
         * @param {$protobuf.Writer} [writer] Writer to encode to
         * @returns {$protobuf.Writer} Writer
         */
        LiveStat.encodeDelimited = function encodeDelimited(message, writer) {
            return this.encode(message, writer && writer.len ? writer.fork() : writer).ldelim();
        };

        /**
         * Decodes a LiveStat message from the specified reader or buffer.
         * @function decode
         * @memberof velobench.LiveStat
         * @static
         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
         * @param {number} [length] Message length if known beforehand
         * @returns {velobench.LiveStat} LiveStat
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        LiveStat.decode = function decode(reader, length, error, long) {
            if (!(reader instanceof $Reader))
                reader = $Reader.create(reader);
            if (long === undefined)
                long = 0;
            if (long > $Reader.recursionLimit)
                throw Error("maximum nesting depth exceeded");
            let end, message;
            if (length === undefined)
                end = reader.len;
            else {
                end = reader.pos + length;
                if (end > reader.len)
                    throw RangeError("index out of range");
                length = reader.len;
                reader.len = end;
            }
            message = new $root.velobench.LiveStat();
            while (reader.pos < end) {
                let tag = reader.uint32();
                if (tag === error)
                    break;
                switch (tag >>> 3) {
                case 1: {
                        message.tokS = reader.double();
                        break;
                    }
                case 2: {
                        message.avg = reader.double();
                        break;
                    }
                case 3: {
                        message.min = reader.double();
                        break;
                    }
                case 4: {
                        message.median = reader.double();
                        break;
                    }
                case 5: {
                        message.max = reader.double();
                        break;
                    }
                case 6: {
                        message.tokens = reader.double();
                        break;
                    }
                case 7: {
                        message.ttftMs = reader.double();
                        break;
                    }
                case 8: {
                        message.genMs = reader.double();
                        break;
                    }
                case 9: {
                        message.reasoningTokens = reader.double();
                        break;
                    }
                case 10: {
                        message.contentTokens = reader.double();
                        break;
                    }
                default:
                    reader.skipType(tag & 7, long);
                    break;
                }
            }
            if (length !== undefined) {
                if (reader.pos !== end)
                    throw RangeError("index out of range");
                reader.len = length;
            }
            return message;
        };

        /**
         * Decodes a LiveStat message from the specified reader or buffer, length delimited.
         * @function decodeDelimited
         * @memberof velobench.LiveStat
         * @static
         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
         * @returns {velobench.LiveStat} LiveStat
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        LiveStat.decodeDelimited = function decodeDelimited(reader) {
            if (!(reader instanceof $Reader))
                reader = new $Reader(reader);
            return this.decode(reader, reader.uint32());
        };

        /**
         * Verifies a LiveStat message.
         * @function verify
         * @memberof velobench.LiveStat
         * @static
         * @param {Object.<string,*>} message Plain object to verify
         * @returns {string|null} `null` if valid, otherwise the reason why it is not
         */
        LiveStat.verify = function verify(message, long) {
            if (typeof message !== "object" || message === null)
                return "object expected";
            if (long === undefined)
                long = 0;
            if (long > $util.recursionLimit)
                return "maximum nesting depth exceeded";
            if (message.tokS != null && Object.hasOwnProperty.call(message, "tokS"))
                if (typeof message.tokS !== "number")
                    return "tokS: number expected";
            if (message.avg != null && Object.hasOwnProperty.call(message, "avg"))
                if (typeof message.avg !== "number")
                    return "avg: number expected";
            if (message.min != null && Object.hasOwnProperty.call(message, "min"))
                if (typeof message.min !== "number")
                    return "min: number expected";
            if (message.median != null && Object.hasOwnProperty.call(message, "median"))
                if (typeof message.median !== "number")
                    return "median: number expected";
            if (message.max != null && Object.hasOwnProperty.call(message, "max"))
                if (typeof message.max !== "number")
                    return "max: number expected";
            if (message.tokens != null && Object.hasOwnProperty.call(message, "tokens"))
                if (typeof message.tokens !== "number")
                    return "tokens: number expected";
            if (message.ttftMs != null && Object.hasOwnProperty.call(message, "ttftMs"))
                if (typeof message.ttftMs !== "number")
                    return "ttftMs: number expected";
            if (message.genMs != null && Object.hasOwnProperty.call(message, "genMs"))
                if (typeof message.genMs !== "number")
                    return "genMs: number expected";
            if (message.reasoningTokens != null && Object.hasOwnProperty.call(message, "reasoningTokens"))
                if (typeof message.reasoningTokens !== "number")
                    return "reasoningTokens: number expected";
            if (message.contentTokens != null && Object.hasOwnProperty.call(message, "contentTokens"))
                if (typeof message.contentTokens !== "number")
                    return "contentTokens: number expected";
            return null;
        };

        /**
         * Creates a LiveStat message from a plain object. Also converts values to their respective internal types.
         * @function fromObject
         * @memberof velobench.LiveStat
         * @static
         * @param {Object.<string,*>} object Plain object
         * @returns {velobench.LiveStat} LiveStat
         */
        LiveStat.fromObject = function fromObject(object, long) {
            if (object instanceof $root.velobench.LiveStat)
                return object;
            if (!$util.isObject(object))
                throw TypeError(".velobench.LiveStat: object expected");
            if (long === undefined)
                long = 0;
            if (long > $util.recursionLimit)
                throw Error("maximum nesting depth exceeded");
            let message = new $root.velobench.LiveStat();
            if (object.tokS != null)
                message.tokS = Number(object.tokS);
            if (object.avg != null)
                message.avg = Number(object.avg);
            if (object.min != null)
                message.min = Number(object.min);
            if (object.median != null)
                message.median = Number(object.median);
            if (object.max != null)
                message.max = Number(object.max);
            if (object.tokens != null)
                message.tokens = Number(object.tokens);
            if (object.ttftMs != null)
                message.ttftMs = Number(object.ttftMs);
            if (object.genMs != null)
                message.genMs = Number(object.genMs);
            if (object.reasoningTokens != null)
                message.reasoningTokens = Number(object.reasoningTokens);
            if (object.contentTokens != null)
                message.contentTokens = Number(object.contentTokens);
            return message;
        };

        /**
         * Creates a plain object from a LiveStat message. Also converts values to other types if specified.
         * @function toObject
         * @memberof velobench.LiveStat
         * @static
         * @param {velobench.LiveStat} message LiveStat
         * @param {$protobuf.IConversionOptions} [options] Conversion options
         * @returns {Object.<string,*>} Plain object
         */
        LiveStat.toObject = function toObject(message, options, q) {
            if (!options)
                options = {};
            if (q === undefined)
                q = 0;
            if (q > $util.recursionLimit)
                throw Error("max depth exceeded");
            let object = {};
            if (options.defaults) {
                object.tokS = 0;
                object.avg = 0;
                object.min = 0;
                object.median = 0;
                object.max = 0;
                object.tokens = 0;
                object.ttftMs = 0;
                object.genMs = 0;
                object.reasoningTokens = 0;
                object.contentTokens = 0;
            }
            if (message.tokS != null && Object.hasOwnProperty.call(message, "tokS"))
                object.tokS = options.json && !isFinite(message.tokS) ? String(message.tokS) : message.tokS;
            if (message.avg != null && Object.hasOwnProperty.call(message, "avg"))
                object.avg = options.json && !isFinite(message.avg) ? String(message.avg) : message.avg;
            if (message.min != null && Object.hasOwnProperty.call(message, "min"))
                object.min = options.json && !isFinite(message.min) ? String(message.min) : message.min;
            if (message.median != null && Object.hasOwnProperty.call(message, "median"))
                object.median = options.json && !isFinite(message.median) ? String(message.median) : message.median;
            if (message.max != null && Object.hasOwnProperty.call(message, "max"))
                object.max = options.json && !isFinite(message.max) ? String(message.max) : message.max;
            if (message.tokens != null && Object.hasOwnProperty.call(message, "tokens"))
                object.tokens = options.json && !isFinite(message.tokens) ? String(message.tokens) : message.tokens;
            if (message.ttftMs != null && Object.hasOwnProperty.call(message, "ttftMs"))
                object.ttftMs = options.json && !isFinite(message.ttftMs) ? String(message.ttftMs) : message.ttftMs;
            if (message.genMs != null && Object.hasOwnProperty.call(message, "genMs"))
                object.genMs = options.json && !isFinite(message.genMs) ? String(message.genMs) : message.genMs;
            if (message.reasoningTokens != null && Object.hasOwnProperty.call(message, "reasoningTokens"))
                object.reasoningTokens = options.json && !isFinite(message.reasoningTokens) ? String(message.reasoningTokens) : message.reasoningTokens;
            if (message.contentTokens != null && Object.hasOwnProperty.call(message, "contentTokens"))
                object.contentTokens = options.json && !isFinite(message.contentTokens) ? String(message.contentTokens) : message.contentTokens;
            return object;
        };

        /**
         * Converts this LiveStat to JSON.
         * @function toJSON
         * @memberof velobench.LiveStat
         * @instance
         * @returns {Object.<string,*>} JSON object
         */
        LiveStat.prototype.toJSON = function toJSON() {
            return this.constructor.toObject(this, $protobuf.util.toJSONOptions);
        };

        /**
         * Gets the default type url for LiveStat
         * @function getTypeUrl
         * @memberof velobench.LiveStat
         * @static
         * @param {string} [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns {string} The default type url
         */
        LiveStat.getTypeUrl = function getTypeUrl(typeUrlPrefix) {
            if (typeUrlPrefix === undefined) {
                typeUrlPrefix = "type.googleapis.com";
            }
            return typeUrlPrefix + "/velobench.LiveStat";
        };

        return LiveStat;
    })();

    velobench.Stats = (function() {

        /**
         * Properties of a Stats.
         * @memberof velobench
         * @interface IStats
         * @property {Array.<velobench.IDecodePoint>|null} [decode] Stats decode
         * @property {Array.<number>|null} [latencies] Stats latencies
         * @property {velobench.ILiveStat|null} [live] Stats live
         * @property {velobench.IClusterResult|null} [clusters] Stats clusters
         * @property {Array.<velobench.IAcceptancePoint>|null} [acceptance] Stats acceptance
         * @property {Array.<velobench.ISpecDepthPoint>|null} [specDepth] Stats specDepth
         * @property {Array.<velobench.IRegime>|null} [regimes] Stats regimes
         * @property {string|null} [category] Stats category
         * @property {number|null} [histMax] Stats histMax
         */

        /**
         * Constructs a new Stats.
         * @memberof velobench
         * @classdesc Represents a Stats.
         * @implements IStats
         * @constructor
         * @param {velobench.IStats=} [properties] Properties to set
         */
        function Stats(properties) {
            this.decode = [];
            this.latencies = [];
            this.acceptance = [];
            this.specDepth = [];
            this.regimes = [];
            if (properties)
                for (let keys = Object.keys(properties), i = 0; i < keys.length; ++i)
                    if (properties[keys[i]] != null && keys[i] !== "__proto__")
                        this[keys[i]] = properties[keys[i]];
        }

        /**
         * Stats decode.
         * @member {Array.<velobench.IDecodePoint>} decode
         * @memberof velobench.Stats
         * @instance
         */
        Stats.prototype.decode = $util.emptyArray;

        /**
         * Stats latencies.
         * @member {Array.<number>} latencies
         * @memberof velobench.Stats
         * @instance
         */
        Stats.prototype.latencies = $util.emptyArray;

        /**
         * Stats live.
         * @member {velobench.ILiveStat|null|undefined} live
         * @memberof velobench.Stats
         * @instance
         */
        Stats.prototype.live = null;

        /**
         * Stats clusters.
         * @member {velobench.IClusterResult|null|undefined} clusters
         * @memberof velobench.Stats
         * @instance
         */
        Stats.prototype.clusters = null;

        /**
         * Stats acceptance.
         * @member {Array.<velobench.IAcceptancePoint>} acceptance
         * @memberof velobench.Stats
         * @instance
         */
        Stats.prototype.acceptance = $util.emptyArray;

        /**
         * Stats specDepth.
         * @member {Array.<velobench.ISpecDepthPoint>} specDepth
         * @memberof velobench.Stats
         * @instance
         */
        Stats.prototype.specDepth = $util.emptyArray;

        /**
         * Stats regimes.
         * @member {Array.<velobench.IRegime>} regimes
         * @memberof velobench.Stats
         * @instance
         */
        Stats.prototype.regimes = $util.emptyArray;

        /**
         * Stats category.
         * @member {string} category
         * @memberof velobench.Stats
         * @instance
         */
        Stats.prototype.category = "";

        /**
         * Stats histMax.
         * @member {number} histMax
         * @memberof velobench.Stats
         * @instance
         */
        Stats.prototype.histMax = 0;

        /**
         * Creates a new Stats instance using the specified properties.
         * @function create
         * @memberof velobench.Stats
         * @static
         * @param {velobench.IStats=} [properties] Properties to set
         * @returns {velobench.Stats} Stats instance
         */
        Stats.create = function create(properties) {
            return new Stats(properties);
        };

        /**
         * Encodes the specified Stats message. Does not implicitly {@link velobench.Stats.verify|verify} messages.
         * @function encode
         * @memberof velobench.Stats
         * @static
         * @param {velobench.IStats} message Stats message or plain object to encode
         * @param {$protobuf.Writer} [writer] Writer to encode to
         * @returns {$protobuf.Writer} Writer
         */
        Stats.encode = function encode(message, writer, q) {
            if (!writer)
                writer = $Writer.create();
            if (q === undefined)
                q = 0;
            if (q > $util.recursionLimit)
                throw Error("max depth exceeded");
            if (message.decode != null && message.decode.length)
                for (let i = 0; i < message.decode.length; ++i)
                    $root.velobench.DecodePoint.encode(message.decode[i], writer.uint32(/* id 1, wireType 2 =*/10).fork(), q + 1).ldelim();
            if (message.latencies != null && message.latencies.length) {
                writer.uint32(/* id 2, wireType 2 =*/18).fork();
                for (let i = 0; i < message.latencies.length; ++i)
                    writer.double(message.latencies[i]);
                writer.ldelim();
            }
            if (message.live != null && Object.hasOwnProperty.call(message, "live"))
                $root.velobench.LiveStat.encode(message.live, writer.uint32(/* id 3, wireType 2 =*/26).fork(), q + 1).ldelim();
            if (message.clusters != null && Object.hasOwnProperty.call(message, "clusters"))
                $root.velobench.ClusterResult.encode(message.clusters, writer.uint32(/* id 4, wireType 2 =*/34).fork(), q + 1).ldelim();
            if (message.acceptance != null && message.acceptance.length)
                for (let i = 0; i < message.acceptance.length; ++i)
                    $root.velobench.AcceptancePoint.encode(message.acceptance[i], writer.uint32(/* id 5, wireType 2 =*/42).fork(), q + 1).ldelim();
            if (message.specDepth != null && message.specDepth.length)
                for (let i = 0; i < message.specDepth.length; ++i)
                    $root.velobench.SpecDepthPoint.encode(message.specDepth[i], writer.uint32(/* id 6, wireType 2 =*/50).fork(), q + 1).ldelim();
            if (message.regimes != null && message.regimes.length)
                for (let i = 0; i < message.regimes.length; ++i)
                    $root.velobench.Regime.encode(message.regimes[i], writer.uint32(/* id 7, wireType 2 =*/58).fork(), q + 1).ldelim();
            if (message.category != null && Object.hasOwnProperty.call(message, "category"))
                writer.uint32(/* id 8, wireType 2 =*/66).string(message.category);
            if (message.histMax != null && Object.hasOwnProperty.call(message, "histMax"))
                writer.uint32(/* id 9, wireType 1 =*/73).double(message.histMax);
            return writer;
        };

        /**
         * Encodes the specified Stats message, length delimited. Does not implicitly {@link velobench.Stats.verify|verify} messages.
         * @function encodeDelimited
         * @memberof velobench.Stats
         * @static
         * @param {velobench.IStats} message Stats message or plain object to encode
         * @param {$protobuf.Writer} [writer] Writer to encode to
         * @returns {$protobuf.Writer} Writer
         */
        Stats.encodeDelimited = function encodeDelimited(message, writer) {
            return this.encode(message, writer && writer.len ? writer.fork() : writer).ldelim();
        };

        /**
         * Decodes a Stats message from the specified reader or buffer.
         * @function decode
         * @memberof velobench.Stats
         * @static
         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
         * @param {number} [length] Message length if known beforehand
         * @returns {velobench.Stats} Stats
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        Stats.decode = function decode(reader, length, error, long) {
            if (!(reader instanceof $Reader))
                reader = $Reader.create(reader);
            if (long === undefined)
                long = 0;
            if (long > $Reader.recursionLimit)
                throw Error("maximum nesting depth exceeded");
            let end, message;
            if (length === undefined)
                end = reader.len;
            else {
                end = reader.pos + length;
                if (end > reader.len)
                    throw RangeError("index out of range");
                length = reader.len;
                reader.len = end;
            }
            message = new $root.velobench.Stats();
            while (reader.pos < end) {
                let tag = reader.uint32();
                if (tag === error)
                    break;
                switch (tag >>> 3) {
                case 1: {
                        if (!(message.decode && message.decode.length))
                            message.decode = [];
                        message.decode.push($root.velobench.DecodePoint.decode(reader, reader.uint32(), undefined, long + 1));
                        break;
                    }
                case 2: {
                        if (!(message.latencies && message.latencies.length))
                            message.latencies = [];
                        if ((tag & 7) === 2) {
                            let end2 = reader.uint32() + reader.pos;
                            if (end2 > reader.len)
                                throw RangeError("index out of range");
                            reader.len = end2;
                            while (reader.pos < end2)
                                message.latencies.push(reader.double());
                            if (reader.pos !== end2)
                                throw RangeError("index out of range");
                            reader.len = end;
                        } else
                            message.latencies.push(reader.double());
                        break;
                    }
                case 3: {
                        message.live = $root.velobench.LiveStat.decode(reader, reader.uint32(), undefined, long + 1);
                        break;
                    }
                case 4: {
                        message.clusters = $root.velobench.ClusterResult.decode(reader, reader.uint32(), undefined, long + 1);
                        break;
                    }
                case 5: {
                        if (!(message.acceptance && message.acceptance.length))
                            message.acceptance = [];
                        message.acceptance.push($root.velobench.AcceptancePoint.decode(reader, reader.uint32(), undefined, long + 1));
                        break;
                    }
                case 6: {
                        if (!(message.specDepth && message.specDepth.length))
                            message.specDepth = [];
                        message.specDepth.push($root.velobench.SpecDepthPoint.decode(reader, reader.uint32(), undefined, long + 1));
                        break;
                    }
                case 7: {
                        if (!(message.regimes && message.regimes.length))
                            message.regimes = [];
                        message.regimes.push($root.velobench.Regime.decode(reader, reader.uint32(), undefined, long + 1));
                        break;
                    }
                case 8: {
                        message.category = reader.string();
                        break;
                    }
                case 9: {
                        message.histMax = reader.double();
                        break;
                    }
                default:
                    reader.skipType(tag & 7, long);
                    break;
                }
            }
            if (length !== undefined) {
                if (reader.pos !== end)
                    throw RangeError("index out of range");
                reader.len = length;
            }
            return message;
        };

        /**
         * Decodes a Stats message from the specified reader or buffer, length delimited.
         * @function decodeDelimited
         * @memberof velobench.Stats
         * @static
         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
         * @returns {velobench.Stats} Stats
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        Stats.decodeDelimited = function decodeDelimited(reader) {
            if (!(reader instanceof $Reader))
                reader = new $Reader(reader);
            return this.decode(reader, reader.uint32());
        };

        /**
         * Verifies a Stats message.
         * @function verify
         * @memberof velobench.Stats
         * @static
         * @param {Object.<string,*>} message Plain object to verify
         * @returns {string|null} `null` if valid, otherwise the reason why it is not
         */
        Stats.verify = function verify(message, long) {
            if (typeof message !== "object" || message === null)
                return "object expected";
            if (long === undefined)
                long = 0;
            if (long > $util.recursionLimit)
                return "maximum nesting depth exceeded";
            if (message.decode != null && Object.hasOwnProperty.call(message, "decode")) {
                if (!Array.isArray(message.decode))
                    return "decode: array expected";
                for (let i = 0; i < message.decode.length; ++i) {
                    let error = $root.velobench.DecodePoint.verify(message.decode[i], long + 1);
                    if (error)
                        return "decode." + error;
                }
            }
            if (message.latencies != null && Object.hasOwnProperty.call(message, "latencies")) {
                if (!Array.isArray(message.latencies))
                    return "latencies: array expected";
                for (let i = 0; i < message.latencies.length; ++i)
                    if (typeof message.latencies[i] !== "number")
                        return "latencies: number[] expected";
            }
            if (message.live != null && Object.hasOwnProperty.call(message, "live")) {
                let error = $root.velobench.LiveStat.verify(message.live, long + 1);
                if (error)
                    return "live." + error;
            }
            if (message.clusters != null && Object.hasOwnProperty.call(message, "clusters")) {
                let error = $root.velobench.ClusterResult.verify(message.clusters, long + 1);
                if (error)
                    return "clusters." + error;
            }
            if (message.acceptance != null && Object.hasOwnProperty.call(message, "acceptance")) {
                if (!Array.isArray(message.acceptance))
                    return "acceptance: array expected";
                for (let i = 0; i < message.acceptance.length; ++i) {
                    let error = $root.velobench.AcceptancePoint.verify(message.acceptance[i], long + 1);
                    if (error)
                        return "acceptance." + error;
                }
            }
            if (message.specDepth != null && Object.hasOwnProperty.call(message, "specDepth")) {
                if (!Array.isArray(message.specDepth))
                    return "specDepth: array expected";
                for (let i = 0; i < message.specDepth.length; ++i) {
                    let error = $root.velobench.SpecDepthPoint.verify(message.specDepth[i], long + 1);
                    if (error)
                        return "specDepth." + error;
                }
            }
            if (message.regimes != null && Object.hasOwnProperty.call(message, "regimes")) {
                if (!Array.isArray(message.regimes))
                    return "regimes: array expected";
                for (let i = 0; i < message.regimes.length; ++i) {
                    let error = $root.velobench.Regime.verify(message.regimes[i], long + 1);
                    if (error)
                        return "regimes." + error;
                }
            }
            if (message.category != null && Object.hasOwnProperty.call(message, "category"))
                if (!$util.isString(message.category))
                    return "category: string expected";
            if (message.histMax != null && Object.hasOwnProperty.call(message, "histMax"))
                if (typeof message.histMax !== "number")
                    return "histMax: number expected";
            return null;
        };

        /**
         * Creates a Stats message from a plain object. Also converts values to their respective internal types.
         * @function fromObject
         * @memberof velobench.Stats
         * @static
         * @param {Object.<string,*>} object Plain object
         * @returns {velobench.Stats} Stats
         */
        Stats.fromObject = function fromObject(object, long) {
            if (object instanceof $root.velobench.Stats)
                return object;
            if (!$util.isObject(object))
                throw TypeError(".velobench.Stats: object expected");
            if (long === undefined)
                long = 0;
            if (long > $util.recursionLimit)
                throw Error("maximum nesting depth exceeded");
            let message = new $root.velobench.Stats();
            if (object.decode) {
                if (!Array.isArray(object.decode))
                    throw TypeError(".velobench.Stats.decode: array expected");
                message.decode = [];
                for (let i = 0; i < object.decode.length; ++i) {
                    if (!$util.isObject(object.decode[i]))
                        throw TypeError(".velobench.Stats.decode: object expected");
                    message.decode[i] = $root.velobench.DecodePoint.fromObject(object.decode[i], long + 1);
                }
            }
            if (object.latencies) {
                if (!Array.isArray(object.latencies))
                    throw TypeError(".velobench.Stats.latencies: array expected");
                message.latencies = [];
                for (let i = 0; i < object.latencies.length; ++i)
                    message.latencies[i] = Number(object.latencies[i]);
            }
            if (object.live != null) {
                if (!$util.isObject(object.live))
                    throw TypeError(".velobench.Stats.live: object expected");
                message.live = $root.velobench.LiveStat.fromObject(object.live, long + 1);
            }
            if (object.clusters != null) {
                if (!$util.isObject(object.clusters))
                    throw TypeError(".velobench.Stats.clusters: object expected");
                message.clusters = $root.velobench.ClusterResult.fromObject(object.clusters, long + 1);
            }
            if (object.acceptance) {
                if (!Array.isArray(object.acceptance))
                    throw TypeError(".velobench.Stats.acceptance: array expected");
                message.acceptance = [];
                for (let i = 0; i < object.acceptance.length; ++i) {
                    if (!$util.isObject(object.acceptance[i]))
                        throw TypeError(".velobench.Stats.acceptance: object expected");
                    message.acceptance[i] = $root.velobench.AcceptancePoint.fromObject(object.acceptance[i], long + 1);
                }
            }
            if (object.specDepth) {
                if (!Array.isArray(object.specDepth))
                    throw TypeError(".velobench.Stats.specDepth: array expected");
                message.specDepth = [];
                for (let i = 0; i < object.specDepth.length; ++i) {
                    if (!$util.isObject(object.specDepth[i]))
                        throw TypeError(".velobench.Stats.specDepth: object expected");
                    message.specDepth[i] = $root.velobench.SpecDepthPoint.fromObject(object.specDepth[i], long + 1);
                }
            }
            if (object.regimes) {
                if (!Array.isArray(object.regimes))
                    throw TypeError(".velobench.Stats.regimes: array expected");
                message.regimes = [];
                for (let i = 0; i < object.regimes.length; ++i) {
                    if (!$util.isObject(object.regimes[i]))
                        throw TypeError(".velobench.Stats.regimes: object expected");
                    message.regimes[i] = $root.velobench.Regime.fromObject(object.regimes[i], long + 1);
                }
            }
            if (object.category != null)
                message.category = String(object.category);
            if (object.histMax != null)
                message.histMax = Number(object.histMax);
            return message;
        };

        /**
         * Creates a plain object from a Stats message. Also converts values to other types if specified.
         * @function toObject
         * @memberof velobench.Stats
         * @static
         * @param {velobench.Stats} message Stats
         * @param {$protobuf.IConversionOptions} [options] Conversion options
         * @returns {Object.<string,*>} Plain object
         */
        Stats.toObject = function toObject(message, options, q) {
            if (!options)
                options = {};
            if (q === undefined)
                q = 0;
            if (q > $util.recursionLimit)
                throw Error("max depth exceeded");
            let object = {};
            if (options.arrays || options.defaults) {
                object.decode = [];
                object.latencies = [];
                object.acceptance = [];
                object.specDepth = [];
                object.regimes = [];
            }
            if (options.defaults) {
                object.live = null;
                object.clusters = null;
                object.category = "";
                object.histMax = 0;
            }
            if (message.decode && message.decode.length) {
                object.decode = [];
                for (let j = 0; j < message.decode.length; ++j)
                    object.decode[j] = $root.velobench.DecodePoint.toObject(message.decode[j], options, q + 1);
            }
            if (message.latencies && message.latencies.length) {
                object.latencies = [];
                for (let j = 0; j < message.latencies.length; ++j)
                    object.latencies[j] = options.json && !isFinite(message.latencies[j]) ? String(message.latencies[j]) : message.latencies[j];
            }
            if (message.live != null && Object.hasOwnProperty.call(message, "live"))
                object.live = $root.velobench.LiveStat.toObject(message.live, options, q + 1);
            if (message.clusters != null && Object.hasOwnProperty.call(message, "clusters"))
                object.clusters = $root.velobench.ClusterResult.toObject(message.clusters, options, q + 1);
            if (message.acceptance && message.acceptance.length) {
                object.acceptance = [];
                for (let j = 0; j < message.acceptance.length; ++j)
                    object.acceptance[j] = $root.velobench.AcceptancePoint.toObject(message.acceptance[j], options, q + 1);
            }
            if (message.specDepth && message.specDepth.length) {
                object.specDepth = [];
                for (let j = 0; j < message.specDepth.length; ++j)
                    object.specDepth[j] = $root.velobench.SpecDepthPoint.toObject(message.specDepth[j], options, q + 1);
            }
            if (message.regimes && message.regimes.length) {
                object.regimes = [];
                for (let j = 0; j < message.regimes.length; ++j)
                    object.regimes[j] = $root.velobench.Regime.toObject(message.regimes[j], options, q + 1);
            }
            if (message.category != null && Object.hasOwnProperty.call(message, "category"))
                object.category = message.category;
            if (message.histMax != null && Object.hasOwnProperty.call(message, "histMax"))
                object.histMax = options.json && !isFinite(message.histMax) ? String(message.histMax) : message.histMax;
            return object;
        };

        /**
         * Converts this Stats to JSON.
         * @function toJSON
         * @memberof velobench.Stats
         * @instance
         * @returns {Object.<string,*>} JSON object
         */
        Stats.prototype.toJSON = function toJSON() {
            return this.constructor.toObject(this, $protobuf.util.toJSONOptions);
        };

        /**
         * Gets the default type url for Stats
         * @function getTypeUrl
         * @memberof velobench.Stats
         * @static
         * @param {string} [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns {string} The default type url
         */
        Stats.getTypeUrl = function getTypeUrl(typeUrlPrefix) {
            if (typeUrlPrefix === undefined) {
                typeUrlPrefix = "type.googleapis.com";
            }
            return typeUrlPrefix + "/velobench.Stats";
        };

        return Stats;
    })();

    velobench.Regime = (function() {

        /**
         * Properties of a Regime.
         * @memberof velobench
         * @interface IRegime
         * @property {string|null} [category] Regime category
         * @property {number|null} [tokenCount] Regime tokenCount
         * @property {number|null} [avgTokS] Regime avgTokS
         * @property {number|null} [minTokS] Regime minTokS
         * @property {number|null} [medianTokS] Regime medianTokS
         * @property {number|null} [maxTokS] Regime maxTokS
         * @property {Array.<velobench.IDecodePoint>|null} [samples] Regime samples
         */

        /**
         * Constructs a new Regime.
         * @memberof velobench
         * @classdesc Represents a Regime.
         * @implements IRegime
         * @constructor
         * @param {velobench.IRegime=} [properties] Properties to set
         */
        function Regime(properties) {
            this.samples = [];
            if (properties)
                for (let keys = Object.keys(properties), i = 0; i < keys.length; ++i)
                    if (properties[keys[i]] != null && keys[i] !== "__proto__")
                        this[keys[i]] = properties[keys[i]];
        }

        /**
         * Regime category.
         * @member {string} category
         * @memberof velobench.Regime
         * @instance
         */
        Regime.prototype.category = "";

        /**
         * Regime tokenCount.
         * @member {number} tokenCount
         * @memberof velobench.Regime
         * @instance
         */
        Regime.prototype.tokenCount = 0;

        /**
         * Regime avgTokS.
         * @member {number} avgTokS
         * @memberof velobench.Regime
         * @instance
         */
        Regime.prototype.avgTokS = 0;

        /**
         * Regime minTokS.
         * @member {number} minTokS
         * @memberof velobench.Regime
         * @instance
         */
        Regime.prototype.minTokS = 0;

        /**
         * Regime medianTokS.
         * @member {number} medianTokS
         * @memberof velobench.Regime
         * @instance
         */
        Regime.prototype.medianTokS = 0;

        /**
         * Regime maxTokS.
         * @member {number} maxTokS
         * @memberof velobench.Regime
         * @instance
         */
        Regime.prototype.maxTokS = 0;

        /**
         * Regime samples.
         * @member {Array.<velobench.IDecodePoint>} samples
         * @memberof velobench.Regime
         * @instance
         */
        Regime.prototype.samples = $util.emptyArray;

        /**
         * Creates a new Regime instance using the specified properties.
         * @function create
         * @memberof velobench.Regime
         * @static
         * @param {velobench.IRegime=} [properties] Properties to set
         * @returns {velobench.Regime} Regime instance
         */
        Regime.create = function create(properties) {
            return new Regime(properties);
        };

        /**
         * Encodes the specified Regime message. Does not implicitly {@link velobench.Regime.verify|verify} messages.
         * @function encode
         * @memberof velobench.Regime
         * @static
         * @param {velobench.IRegime} message Regime message or plain object to encode
         * @param {$protobuf.Writer} [writer] Writer to encode to
         * @returns {$protobuf.Writer} Writer
         */
        Regime.encode = function encode(message, writer, q) {
            if (!writer)
                writer = $Writer.create();
            if (q === undefined)
                q = 0;
            if (q > $util.recursionLimit)
                throw Error("max depth exceeded");
            if (message.category != null && Object.hasOwnProperty.call(message, "category"))
                writer.uint32(/* id 1, wireType 2 =*/10).string(message.category);
            if (message.tokenCount != null && Object.hasOwnProperty.call(message, "tokenCount"))
                writer.uint32(/* id 2, wireType 1 =*/17).double(message.tokenCount);
            if (message.avgTokS != null && Object.hasOwnProperty.call(message, "avgTokS"))
                writer.uint32(/* id 3, wireType 1 =*/25).double(message.avgTokS);
            if (message.minTokS != null && Object.hasOwnProperty.call(message, "minTokS"))
                writer.uint32(/* id 4, wireType 1 =*/33).double(message.minTokS);
            if (message.medianTokS != null && Object.hasOwnProperty.call(message, "medianTokS"))
                writer.uint32(/* id 5, wireType 1 =*/41).double(message.medianTokS);
            if (message.maxTokS != null && Object.hasOwnProperty.call(message, "maxTokS"))
                writer.uint32(/* id 6, wireType 1 =*/49).double(message.maxTokS);
            if (message.samples != null && message.samples.length)
                for (let i = 0; i < message.samples.length; ++i)
                    $root.velobench.DecodePoint.encode(message.samples[i], writer.uint32(/* id 7, wireType 2 =*/58).fork(), q + 1).ldelim();
            return writer;
        };

        /**
         * Encodes the specified Regime message, length delimited. Does not implicitly {@link velobench.Regime.verify|verify} messages.
         * @function encodeDelimited
         * @memberof velobench.Regime
         * @static
         * @param {velobench.IRegime} message Regime message or plain object to encode
         * @param {$protobuf.Writer} [writer] Writer to encode to
         * @returns {$protobuf.Writer} Writer
         */
        Regime.encodeDelimited = function encodeDelimited(message, writer) {
            return this.encode(message, writer && writer.len ? writer.fork() : writer).ldelim();
        };

        /**
         * Decodes a Regime message from the specified reader or buffer.
         * @function decode
         * @memberof velobench.Regime
         * @static
         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
         * @param {number} [length] Message length if known beforehand
         * @returns {velobench.Regime} Regime
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        Regime.decode = function decode(reader, length, error, long) {
            if (!(reader instanceof $Reader))
                reader = $Reader.create(reader);
            if (long === undefined)
                long = 0;
            if (long > $Reader.recursionLimit)
                throw Error("maximum nesting depth exceeded");
            let end, message;
            if (length === undefined)
                end = reader.len;
            else {
                end = reader.pos + length;
                if (end > reader.len)
                    throw RangeError("index out of range");
                length = reader.len;
                reader.len = end;
            }
            message = new $root.velobench.Regime();
            while (reader.pos < end) {
                let tag = reader.uint32();
                if (tag === error)
                    break;
                switch (tag >>> 3) {
                case 1: {
                        message.category = reader.string();
                        break;
                    }
                case 2: {
                        message.tokenCount = reader.double();
                        break;
                    }
                case 3: {
                        message.avgTokS = reader.double();
                        break;
                    }
                case 4: {
                        message.minTokS = reader.double();
                        break;
                    }
                case 5: {
                        message.medianTokS = reader.double();
                        break;
                    }
                case 6: {
                        message.maxTokS = reader.double();
                        break;
                    }
                case 7: {
                        if (!(message.samples && message.samples.length))
                            message.samples = [];
                        message.samples.push($root.velobench.DecodePoint.decode(reader, reader.uint32(), undefined, long + 1));
                        break;
                    }
                default:
                    reader.skipType(tag & 7, long);
                    break;
                }
            }
            if (length !== undefined) {
                if (reader.pos !== end)
                    throw RangeError("index out of range");
                reader.len = length;
            }
            return message;
        };

        /**
         * Decodes a Regime message from the specified reader or buffer, length delimited.
         * @function decodeDelimited
         * @memberof velobench.Regime
         * @static
         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
         * @returns {velobench.Regime} Regime
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        Regime.decodeDelimited = function decodeDelimited(reader) {
            if (!(reader instanceof $Reader))
                reader = new $Reader(reader);
            return this.decode(reader, reader.uint32());
        };

        /**
         * Verifies a Regime message.
         * @function verify
         * @memberof velobench.Regime
         * @static
         * @param {Object.<string,*>} message Plain object to verify
         * @returns {string|null} `null` if valid, otherwise the reason why it is not
         */
        Regime.verify = function verify(message, long) {
            if (typeof message !== "object" || message === null)
                return "object expected";
            if (long === undefined)
                long = 0;
            if (long > $util.recursionLimit)
                return "maximum nesting depth exceeded";
            if (message.category != null && Object.hasOwnProperty.call(message, "category"))
                if (!$util.isString(message.category))
                    return "category: string expected";
            if (message.tokenCount != null && Object.hasOwnProperty.call(message, "tokenCount"))
                if (typeof message.tokenCount !== "number")
                    return "tokenCount: number expected";
            if (message.avgTokS != null && Object.hasOwnProperty.call(message, "avgTokS"))
                if (typeof message.avgTokS !== "number")
                    return "avgTokS: number expected";
            if (message.minTokS != null && Object.hasOwnProperty.call(message, "minTokS"))
                if (typeof message.minTokS !== "number")
                    return "minTokS: number expected";
            if (message.medianTokS != null && Object.hasOwnProperty.call(message, "medianTokS"))
                if (typeof message.medianTokS !== "number")
                    return "medianTokS: number expected";
            if (message.maxTokS != null && Object.hasOwnProperty.call(message, "maxTokS"))
                if (typeof message.maxTokS !== "number")
                    return "maxTokS: number expected";
            if (message.samples != null && Object.hasOwnProperty.call(message, "samples")) {
                if (!Array.isArray(message.samples))
                    return "samples: array expected";
                for (let i = 0; i < message.samples.length; ++i) {
                    let error = $root.velobench.DecodePoint.verify(message.samples[i], long + 1);
                    if (error)
                        return "samples." + error;
                }
            }
            return null;
        };

        /**
         * Creates a Regime message from a plain object. Also converts values to their respective internal types.
         * @function fromObject
         * @memberof velobench.Regime
         * @static
         * @param {Object.<string,*>} object Plain object
         * @returns {velobench.Regime} Regime
         */
        Regime.fromObject = function fromObject(object, long) {
            if (object instanceof $root.velobench.Regime)
                return object;
            if (!$util.isObject(object))
                throw TypeError(".velobench.Regime: object expected");
            if (long === undefined)
                long = 0;
            if (long > $util.recursionLimit)
                throw Error("maximum nesting depth exceeded");
            let message = new $root.velobench.Regime();
            if (object.category != null)
                message.category = String(object.category);
            if (object.tokenCount != null)
                message.tokenCount = Number(object.tokenCount);
            if (object.avgTokS != null)
                message.avgTokS = Number(object.avgTokS);
            if (object.minTokS != null)
                message.minTokS = Number(object.minTokS);
            if (object.medianTokS != null)
                message.medianTokS = Number(object.medianTokS);
            if (object.maxTokS != null)
                message.maxTokS = Number(object.maxTokS);
            if (object.samples) {
                if (!Array.isArray(object.samples))
                    throw TypeError(".velobench.Regime.samples: array expected");
                message.samples = [];
                for (let i = 0; i < object.samples.length; ++i) {
                    if (!$util.isObject(object.samples[i]))
                        throw TypeError(".velobench.Regime.samples: object expected");
                    message.samples[i] = $root.velobench.DecodePoint.fromObject(object.samples[i], long + 1);
                }
            }
            return message;
        };

        /**
         * Creates a plain object from a Regime message. Also converts values to other types if specified.
         * @function toObject
         * @memberof velobench.Regime
         * @static
         * @param {velobench.Regime} message Regime
         * @param {$protobuf.IConversionOptions} [options] Conversion options
         * @returns {Object.<string,*>} Plain object
         */
        Regime.toObject = function toObject(message, options, q) {
            if (!options)
                options = {};
            if (q === undefined)
                q = 0;
            if (q > $util.recursionLimit)
                throw Error("max depth exceeded");
            let object = {};
            if (options.arrays || options.defaults)
                object.samples = [];
            if (options.defaults) {
                object.category = "";
                object.tokenCount = 0;
                object.avgTokS = 0;
                object.minTokS = 0;
                object.medianTokS = 0;
                object.maxTokS = 0;
            }
            if (message.category != null && Object.hasOwnProperty.call(message, "category"))
                object.category = message.category;
            if (message.tokenCount != null && Object.hasOwnProperty.call(message, "tokenCount"))
                object.tokenCount = options.json && !isFinite(message.tokenCount) ? String(message.tokenCount) : message.tokenCount;
            if (message.avgTokS != null && Object.hasOwnProperty.call(message, "avgTokS"))
                object.avgTokS = options.json && !isFinite(message.avgTokS) ? String(message.avgTokS) : message.avgTokS;
            if (message.minTokS != null && Object.hasOwnProperty.call(message, "minTokS"))
                object.minTokS = options.json && !isFinite(message.minTokS) ? String(message.minTokS) : message.minTokS;
            if (message.medianTokS != null && Object.hasOwnProperty.call(message, "medianTokS"))
                object.medianTokS = options.json && !isFinite(message.medianTokS) ? String(message.medianTokS) : message.medianTokS;
            if (message.maxTokS != null && Object.hasOwnProperty.call(message, "maxTokS"))
                object.maxTokS = options.json && !isFinite(message.maxTokS) ? String(message.maxTokS) : message.maxTokS;
            if (message.samples && message.samples.length) {
                object.samples = [];
                for (let j = 0; j < message.samples.length; ++j)
                    object.samples[j] = $root.velobench.DecodePoint.toObject(message.samples[j], options, q + 1);
            }
            return object;
        };

        /**
         * Converts this Regime to JSON.
         * @function toJSON
         * @memberof velobench.Regime
         * @instance
         * @returns {Object.<string,*>} JSON object
         */
        Regime.prototype.toJSON = function toJSON() {
            return this.constructor.toObject(this, $protobuf.util.toJSONOptions);
        };

        /**
         * Gets the default type url for Regime
         * @function getTypeUrl
         * @memberof velobench.Regime
         * @static
         * @param {string} [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns {string} The default type url
         */
        Regime.getTypeUrl = function getTypeUrl(typeUrlPrefix) {
            if (typeUrlPrefix === undefined) {
                typeUrlPrefix = "type.googleapis.com";
            }
            return typeUrlPrefix + "/velobench.Regime";
        };

        return Regime;
    })();

    velobench.Done = (function() {

        /**
         * Properties of a Done.
         * @memberof velobench
         * @interface IDone
         * @property {number|null} [totalMs] Done totalMs
         * @property {number|null} [decodeMs] Done decodeMs
         * @property {number|null} [ttftMs] Done ttftMs
         * @property {number|null} [promptTokens] Done promptTokens
         * @property {number|null} [completionTokens] Done completionTokens
         * @property {number|null} [finalTokS] Done finalTokS
         * @property {number|null} [contentTokens] Done contentTokens
         * @property {number|null} [reasoningTokens] Done reasoningTokens
         * @property {string|null} [meta] Done meta
         * @property {string|null} [error] Done error
         */

        /**
         * Constructs a new Done.
         * @memberof velobench
         * @classdesc Represents a Done.
         * @implements IDone
         * @constructor
         * @param {velobench.IDone=} [properties] Properties to set
         */
        function Done(properties) {
            if (properties)
                for (let keys = Object.keys(properties), i = 0; i < keys.length; ++i)
                    if (properties[keys[i]] != null && keys[i] !== "__proto__")
                        this[keys[i]] = properties[keys[i]];
        }

        /**
         * Done totalMs.
         * @member {number} totalMs
         * @memberof velobench.Done
         * @instance
         */
        Done.prototype.totalMs = 0;

        /**
         * Done decodeMs.
         * @member {number} decodeMs
         * @memberof velobench.Done
         * @instance
         */
        Done.prototype.decodeMs = 0;

        /**
         * Done ttftMs.
         * @member {number} ttftMs
         * @memberof velobench.Done
         * @instance
         */
        Done.prototype.ttftMs = 0;

        /**
         * Done promptTokens.
         * @member {number} promptTokens
         * @memberof velobench.Done
         * @instance
         */
        Done.prototype.promptTokens = 0;

        /**
         * Done completionTokens.
         * @member {number} completionTokens
         * @memberof velobench.Done
         * @instance
         */
        Done.prototype.completionTokens = 0;

        /**
         * Done finalTokS.
         * @member {number} finalTokS
         * @memberof velobench.Done
         * @instance
         */
        Done.prototype.finalTokS = 0;

        /**
         * Done contentTokens.
         * @member {number} contentTokens
         * @memberof velobench.Done
         * @instance
         */
        Done.prototype.contentTokens = 0;

        /**
         * Done reasoningTokens.
         * @member {number} reasoningTokens
         * @memberof velobench.Done
         * @instance
         */
        Done.prototype.reasoningTokens = 0;

        /**
         * Done meta.
         * @member {string} meta
         * @memberof velobench.Done
         * @instance
         */
        Done.prototype.meta = "";

        /**
         * Done error.
         * @member {string} error
         * @memberof velobench.Done
         * @instance
         */
        Done.prototype.error = "";

        /**
         * Creates a new Done instance using the specified properties.
         * @function create
         * @memberof velobench.Done
         * @static
         * @param {velobench.IDone=} [properties] Properties to set
         * @returns {velobench.Done} Done instance
         */
        Done.create = function create(properties) {
            return new Done(properties);
        };

        /**
         * Encodes the specified Done message. Does not implicitly {@link velobench.Done.verify|verify} messages.
         * @function encode
         * @memberof velobench.Done
         * @static
         * @param {velobench.IDone} message Done message or plain object to encode
         * @param {$protobuf.Writer} [writer] Writer to encode to
         * @returns {$protobuf.Writer} Writer
         */
        Done.encode = function encode(message, writer, q) {
            if (!writer)
                writer = $Writer.create();
            if (q === undefined)
                q = 0;
            if (q > $util.recursionLimit)
                throw Error("max depth exceeded");
            if (message.totalMs != null && Object.hasOwnProperty.call(message, "totalMs"))
                writer.uint32(/* id 1, wireType 1 =*/9).double(message.totalMs);
            if (message.decodeMs != null && Object.hasOwnProperty.call(message, "decodeMs"))
                writer.uint32(/* id 2, wireType 1 =*/17).double(message.decodeMs);
            if (message.ttftMs != null && Object.hasOwnProperty.call(message, "ttftMs"))
                writer.uint32(/* id 3, wireType 1 =*/25).double(message.ttftMs);
            if (message.promptTokens != null && Object.hasOwnProperty.call(message, "promptTokens"))
                writer.uint32(/* id 4, wireType 1 =*/33).double(message.promptTokens);
            if (message.completionTokens != null && Object.hasOwnProperty.call(message, "completionTokens"))
                writer.uint32(/* id 5, wireType 1 =*/41).double(message.completionTokens);
            if (message.finalTokS != null && Object.hasOwnProperty.call(message, "finalTokS"))
                writer.uint32(/* id 6, wireType 1 =*/49).double(message.finalTokS);
            if (message.contentTokens != null && Object.hasOwnProperty.call(message, "contentTokens"))
                writer.uint32(/* id 7, wireType 1 =*/57).double(message.contentTokens);
            if (message.reasoningTokens != null && Object.hasOwnProperty.call(message, "reasoningTokens"))
                writer.uint32(/* id 8, wireType 1 =*/65).double(message.reasoningTokens);
            if (message.meta != null && Object.hasOwnProperty.call(message, "meta"))
                writer.uint32(/* id 9, wireType 2 =*/74).string(message.meta);
            if (message.error != null && Object.hasOwnProperty.call(message, "error"))
                writer.uint32(/* id 10, wireType 2 =*/82).string(message.error);
            return writer;
        };

        /**
         * Encodes the specified Done message, length delimited. Does not implicitly {@link velobench.Done.verify|verify} messages.
         * @function encodeDelimited
         * @memberof velobench.Done
         * @static
         * @param {velobench.IDone} message Done message or plain object to encode
         * @param {$protobuf.Writer} [writer] Writer to encode to
         * @returns {$protobuf.Writer} Writer
         */
        Done.encodeDelimited = function encodeDelimited(message, writer) {
            return this.encode(message, writer && writer.len ? writer.fork() : writer).ldelim();
        };

        /**
         * Decodes a Done message from the specified reader or buffer.
         * @function decode
         * @memberof velobench.Done
         * @static
         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
         * @param {number} [length] Message length if known beforehand
         * @returns {velobench.Done} Done
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        Done.decode = function decode(reader, length, error, long) {
            if (!(reader instanceof $Reader))
                reader = $Reader.create(reader);
            if (long === undefined)
                long = 0;
            if (long > $Reader.recursionLimit)
                throw Error("maximum nesting depth exceeded");
            let end, message;
            if (length === undefined)
                end = reader.len;
            else {
                end = reader.pos + length;
                if (end > reader.len)
                    throw RangeError("index out of range");
                length = reader.len;
                reader.len = end;
            }
            message = new $root.velobench.Done();
            while (reader.pos < end) {
                let tag = reader.uint32();
                if (tag === error)
                    break;
                switch (tag >>> 3) {
                case 1: {
                        message.totalMs = reader.double();
                        break;
                    }
                case 2: {
                        message.decodeMs = reader.double();
                        break;
                    }
                case 3: {
                        message.ttftMs = reader.double();
                        break;
                    }
                case 4: {
                        message.promptTokens = reader.double();
                        break;
                    }
                case 5: {
                        message.completionTokens = reader.double();
                        break;
                    }
                case 6: {
                        message.finalTokS = reader.double();
                        break;
                    }
                case 7: {
                        message.contentTokens = reader.double();
                        break;
                    }
                case 8: {
                        message.reasoningTokens = reader.double();
                        break;
                    }
                case 9: {
                        message.meta = reader.string();
                        break;
                    }
                case 10: {
                        message.error = reader.string();
                        break;
                    }
                default:
                    reader.skipType(tag & 7, long);
                    break;
                }
            }
            if (length !== undefined) {
                if (reader.pos !== end)
                    throw RangeError("index out of range");
                reader.len = length;
            }
            return message;
        };

        /**
         * Decodes a Done message from the specified reader or buffer, length delimited.
         * @function decodeDelimited
         * @memberof velobench.Done
         * @static
         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
         * @returns {velobench.Done} Done
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        Done.decodeDelimited = function decodeDelimited(reader) {
            if (!(reader instanceof $Reader))
                reader = new $Reader(reader);
            return this.decode(reader, reader.uint32());
        };

        /**
         * Verifies a Done message.
         * @function verify
         * @memberof velobench.Done
         * @static
         * @param {Object.<string,*>} message Plain object to verify
         * @returns {string|null} `null` if valid, otherwise the reason why it is not
         */
        Done.verify = function verify(message, long) {
            if (typeof message !== "object" || message === null)
                return "object expected";
            if (long === undefined)
                long = 0;
            if (long > $util.recursionLimit)
                return "maximum nesting depth exceeded";
            if (message.totalMs != null && Object.hasOwnProperty.call(message, "totalMs"))
                if (typeof message.totalMs !== "number")
                    return "totalMs: number expected";
            if (message.decodeMs != null && Object.hasOwnProperty.call(message, "decodeMs"))
                if (typeof message.decodeMs !== "number")
                    return "decodeMs: number expected";
            if (message.ttftMs != null && Object.hasOwnProperty.call(message, "ttftMs"))
                if (typeof message.ttftMs !== "number")
                    return "ttftMs: number expected";
            if (message.promptTokens != null && Object.hasOwnProperty.call(message, "promptTokens"))
                if (typeof message.promptTokens !== "number")
                    return "promptTokens: number expected";
            if (message.completionTokens != null && Object.hasOwnProperty.call(message, "completionTokens"))
                if (typeof message.completionTokens !== "number")
                    return "completionTokens: number expected";
            if (message.finalTokS != null && Object.hasOwnProperty.call(message, "finalTokS"))
                if (typeof message.finalTokS !== "number")
                    return "finalTokS: number expected";
            if (message.contentTokens != null && Object.hasOwnProperty.call(message, "contentTokens"))
                if (typeof message.contentTokens !== "number")
                    return "contentTokens: number expected";
            if (message.reasoningTokens != null && Object.hasOwnProperty.call(message, "reasoningTokens"))
                if (typeof message.reasoningTokens !== "number")
                    return "reasoningTokens: number expected";
            if (message.meta != null && Object.hasOwnProperty.call(message, "meta"))
                if (!$util.isString(message.meta))
                    return "meta: string expected";
            if (message.error != null && Object.hasOwnProperty.call(message, "error"))
                if (!$util.isString(message.error))
                    return "error: string expected";
            return null;
        };

        /**
         * Creates a Done message from a plain object. Also converts values to their respective internal types.
         * @function fromObject
         * @memberof velobench.Done
         * @static
         * @param {Object.<string,*>} object Plain object
         * @returns {velobench.Done} Done
         */
        Done.fromObject = function fromObject(object, long) {
            if (object instanceof $root.velobench.Done)
                return object;
            if (!$util.isObject(object))
                throw TypeError(".velobench.Done: object expected");
            if (long === undefined)
                long = 0;
            if (long > $util.recursionLimit)
                throw Error("maximum nesting depth exceeded");
            let message = new $root.velobench.Done();
            if (object.totalMs != null)
                message.totalMs = Number(object.totalMs);
            if (object.decodeMs != null)
                message.decodeMs = Number(object.decodeMs);
            if (object.ttftMs != null)
                message.ttftMs = Number(object.ttftMs);
            if (object.promptTokens != null)
                message.promptTokens = Number(object.promptTokens);
            if (object.completionTokens != null)
                message.completionTokens = Number(object.completionTokens);
            if (object.finalTokS != null)
                message.finalTokS = Number(object.finalTokS);
            if (object.contentTokens != null)
                message.contentTokens = Number(object.contentTokens);
            if (object.reasoningTokens != null)
                message.reasoningTokens = Number(object.reasoningTokens);
            if (object.meta != null)
                message.meta = String(object.meta);
            if (object.error != null)
                message.error = String(object.error);
            return message;
        };

        /**
         * Creates a plain object from a Done message. Also converts values to other types if specified.
         * @function toObject
         * @memberof velobench.Done
         * @static
         * @param {velobench.Done} message Done
         * @param {$protobuf.IConversionOptions} [options] Conversion options
         * @returns {Object.<string,*>} Plain object
         */
        Done.toObject = function toObject(message, options, q) {
            if (!options)
                options = {};
            if (q === undefined)
                q = 0;
            if (q > $util.recursionLimit)
                throw Error("max depth exceeded");
            let object = {};
            if (options.defaults) {
                object.totalMs = 0;
                object.decodeMs = 0;
                object.ttftMs = 0;
                object.promptTokens = 0;
                object.completionTokens = 0;
                object.finalTokS = 0;
                object.contentTokens = 0;
                object.reasoningTokens = 0;
                object.meta = "";
                object.error = "";
            }
            if (message.totalMs != null && Object.hasOwnProperty.call(message, "totalMs"))
                object.totalMs = options.json && !isFinite(message.totalMs) ? String(message.totalMs) : message.totalMs;
            if (message.decodeMs != null && Object.hasOwnProperty.call(message, "decodeMs"))
                object.decodeMs = options.json && !isFinite(message.decodeMs) ? String(message.decodeMs) : message.decodeMs;
            if (message.ttftMs != null && Object.hasOwnProperty.call(message, "ttftMs"))
                object.ttftMs = options.json && !isFinite(message.ttftMs) ? String(message.ttftMs) : message.ttftMs;
            if (message.promptTokens != null && Object.hasOwnProperty.call(message, "promptTokens"))
                object.promptTokens = options.json && !isFinite(message.promptTokens) ? String(message.promptTokens) : message.promptTokens;
            if (message.completionTokens != null && Object.hasOwnProperty.call(message, "completionTokens"))
                object.completionTokens = options.json && !isFinite(message.completionTokens) ? String(message.completionTokens) : message.completionTokens;
            if (message.finalTokS != null && Object.hasOwnProperty.call(message, "finalTokS"))
                object.finalTokS = options.json && !isFinite(message.finalTokS) ? String(message.finalTokS) : message.finalTokS;
            if (message.contentTokens != null && Object.hasOwnProperty.call(message, "contentTokens"))
                object.contentTokens = options.json && !isFinite(message.contentTokens) ? String(message.contentTokens) : message.contentTokens;
            if (message.reasoningTokens != null && Object.hasOwnProperty.call(message, "reasoningTokens"))
                object.reasoningTokens = options.json && !isFinite(message.reasoningTokens) ? String(message.reasoningTokens) : message.reasoningTokens;
            if (message.meta != null && Object.hasOwnProperty.call(message, "meta"))
                object.meta = message.meta;
            if (message.error != null && Object.hasOwnProperty.call(message, "error"))
                object.error = message.error;
            return object;
        };

        /**
         * Converts this Done to JSON.
         * @function toJSON
         * @memberof velobench.Done
         * @instance
         * @returns {Object.<string,*>} JSON object
         */
        Done.prototype.toJSON = function toJSON() {
            return this.constructor.toObject(this, $protobuf.util.toJSONOptions);
        };

        /**
         * Gets the default type url for Done
         * @function getTypeUrl
         * @memberof velobench.Done
         * @static
         * @param {string} [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns {string} The default type url
         */
        Done.getTypeUrl = function getTypeUrl(typeUrlPrefix) {
            if (typeUrlPrefix === undefined) {
                typeUrlPrefix = "type.googleapis.com";
            }
            return typeUrlPrefix + "/velobench.Done";
        };

        return Done;
    })();

    return velobench;
})();

export { $root as default };
