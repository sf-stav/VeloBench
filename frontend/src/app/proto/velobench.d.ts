import * as $protobuf from "protobufjs";
import Long = require("long");
/** Namespace velobench. */
export namespace velobench {

    /** Properties of a ChatRequest. */
    interface IChatRequest {

        /** ChatRequest providerId */
        providerId?: (string|null);

        /** ChatRequest model */
        model?: (string|null);

        /** ChatRequest messages */
        messages?: (velobench.IChatMessage[]|null);

        /** ChatRequest reasoningEnabled */
        reasoningEnabled?: (boolean|null);

        /** ChatRequest reasoningEffort */
        reasoningEffort?: (string|null);

        /** ChatRequest overrides */
        overrides?: (velobench.IParamOverride[]|null);

        /** ChatRequest maxStatsTokens */
        maxStatsTokens?: (number|null);

        /** ChatRequest resetSession */
        resetSession?: (boolean|null);

        /** ChatRequest resetStats */
        resetStats?: (boolean|null);

        /** ChatRequest fillTokens */
        fillTokens?: (number|null);

        /** ChatRequest modelUid */
        modelUid?: (string|null);

        /** ChatRequest kind */
        kind?: (string|null);

        /** ChatRequest label */
        label?: (string|null);

        /** ChatRequest session */
        session?: (string|null);

        /** ChatRequest section */
        section?: (string|null);

        /** ChatRequest regimesFromSections */
        regimesFromSections?: (boolean|null);
    }

    /** Represents a ChatRequest. */
    class ChatRequest implements IChatRequest {

        /**
         * Constructs a new ChatRequest.
         * @param [properties] Properties to set
         */
        constructor(properties?: velobench.IChatRequest);

        /** ChatRequest providerId. */
        public providerId: string;

        /** ChatRequest model. */
        public model: string;

        /** ChatRequest messages. */
        public messages: velobench.IChatMessage[];

        /** ChatRequest reasoningEnabled. */
        public reasoningEnabled: boolean;

        /** ChatRequest reasoningEffort. */
        public reasoningEffort: string;

        /** ChatRequest overrides. */
        public overrides: velobench.IParamOverride[];

        /** ChatRequest maxStatsTokens. */
        public maxStatsTokens: number;

        /** ChatRequest resetSession. */
        public resetSession: boolean;

        /** ChatRequest resetStats. */
        public resetStats: boolean;

        /** ChatRequest fillTokens. */
        public fillTokens: number;

        /** ChatRequest modelUid. */
        public modelUid: string;

        /** ChatRequest kind. */
        public kind: string;

        /** ChatRequest label. */
        public label: string;

        /** ChatRequest session. */
        public session: string;

        /** ChatRequest section. */
        public section: string;

        /** ChatRequest regimesFromSections. */
        public regimesFromSections: boolean;

        /**
         * Creates a new ChatRequest instance using the specified properties.
         * @param [properties] Properties to set
         * @returns ChatRequest instance
         */
        public static create(properties?: velobench.IChatRequest): velobench.ChatRequest;

        /**
         * Encodes the specified ChatRequest message. Does not implicitly {@link velobench.ChatRequest.verify|verify} messages.
         * @param message ChatRequest message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: velobench.IChatRequest, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified ChatRequest message, length delimited. Does not implicitly {@link velobench.ChatRequest.verify|verify} messages.
         * @param message ChatRequest message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: velobench.IChatRequest, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes a ChatRequest message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns ChatRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): velobench.ChatRequest;

        /**
         * Decodes a ChatRequest message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns ChatRequest
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): velobench.ChatRequest;

        /**
         * Verifies a ChatRequest message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates a ChatRequest message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns ChatRequest
         */
        public static fromObject(object: { [k: string]: any }): velobench.ChatRequest;

        /**
         * Creates a plain object from a ChatRequest message. Also converts values to other types if specified.
         * @param message ChatRequest
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: velobench.ChatRequest, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this ChatRequest to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };

        /**
         * Gets the default type url for ChatRequest
         * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns The default type url
         */
        public static getTypeUrl(typeUrlPrefix?: string): string;
    }

    /** Properties of a ChatMessage. */
    interface IChatMessage {

        /** ChatMessage role */
        role?: (string|null);

        /** ChatMessage content */
        content?: (string|null);

        /** ChatMessage images */
        images?: (string[]|null);

        /** ChatMessage fillTokens */
        fillTokens?: (number|null);
    }

    /** Represents a ChatMessage. */
    class ChatMessage implements IChatMessage {

        /**
         * Constructs a new ChatMessage.
         * @param [properties] Properties to set
         */
        constructor(properties?: velobench.IChatMessage);

        /** ChatMessage role. */
        public role: string;

        /** ChatMessage content. */
        public content: string;

        /** ChatMessage images. */
        public images: string[];

        /** ChatMessage fillTokens. */
        public fillTokens: number;

        /**
         * Creates a new ChatMessage instance using the specified properties.
         * @param [properties] Properties to set
         * @returns ChatMessage instance
         */
        public static create(properties?: velobench.IChatMessage): velobench.ChatMessage;

        /**
         * Encodes the specified ChatMessage message. Does not implicitly {@link velobench.ChatMessage.verify|verify} messages.
         * @param message ChatMessage message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: velobench.IChatMessage, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified ChatMessage message, length delimited. Does not implicitly {@link velobench.ChatMessage.verify|verify} messages.
         * @param message ChatMessage message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: velobench.IChatMessage, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes a ChatMessage message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns ChatMessage
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): velobench.ChatMessage;

        /**
         * Decodes a ChatMessage message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns ChatMessage
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): velobench.ChatMessage;

        /**
         * Verifies a ChatMessage message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates a ChatMessage message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns ChatMessage
         */
        public static fromObject(object: { [k: string]: any }): velobench.ChatMessage;

        /**
         * Creates a plain object from a ChatMessage message. Also converts values to other types if specified.
         * @param message ChatMessage
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: velobench.ChatMessage, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this ChatMessage to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };

        /**
         * Gets the default type url for ChatMessage
         * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns The default type url
         */
        public static getTypeUrl(typeUrlPrefix?: string): string;
    }

    /** Properties of a ParamOverride. */
    interface IParamOverride {

        /** ParamOverride key */
        key?: (string|null);

        /** ParamOverride value */
        value?: (string|null);
    }

    /** Represents a ParamOverride. */
    class ParamOverride implements IParamOverride {

        /**
         * Constructs a new ParamOverride.
         * @param [properties] Properties to set
         */
        constructor(properties?: velobench.IParamOverride);

        /** ParamOverride key. */
        public key: string;

        /** ParamOverride value. */
        public value: string;

        /**
         * Creates a new ParamOverride instance using the specified properties.
         * @param [properties] Properties to set
         * @returns ParamOverride instance
         */
        public static create(properties?: velobench.IParamOverride): velobench.ParamOverride;

        /**
         * Encodes the specified ParamOverride message. Does not implicitly {@link velobench.ParamOverride.verify|verify} messages.
         * @param message ParamOverride message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: velobench.IParamOverride, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified ParamOverride message, length delimited. Does not implicitly {@link velobench.ParamOverride.verify|verify} messages.
         * @param message ParamOverride message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: velobench.IParamOverride, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes a ParamOverride message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns ParamOverride
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): velobench.ParamOverride;

        /**
         * Decodes a ParamOverride message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns ParamOverride
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): velobench.ParamOverride;

        /**
         * Verifies a ParamOverride message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates a ParamOverride message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns ParamOverride
         */
        public static fromObject(object: { [k: string]: any }): velobench.ParamOverride;

        /**
         * Creates a plain object from a ParamOverride message. Also converts values to other types if specified.
         * @param message ParamOverride
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: velobench.ParamOverride, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this ParamOverride to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };

        /**
         * Gets the default type url for ParamOverride
         * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns The default type url
         */
        public static getTypeUrl(typeUrlPrefix?: string): string;
    }

    /** Properties of a ServerFrame. */
    interface IServerFrame {

        /** ServerFrame delta */
        delta?: (velobench.IDelta|null);

        /** ServerFrame stats */
        stats?: (velobench.IStats|null);

        /** ServerFrame done */
        done?: (velobench.IDone|null);
    }

    /** Represents a ServerFrame. */
    class ServerFrame implements IServerFrame {

        /**
         * Constructs a new ServerFrame.
         * @param [properties] Properties to set
         */
        constructor(properties?: velobench.IServerFrame);

        /** ServerFrame delta. */
        public delta?: (velobench.IDelta|null);

        /** ServerFrame stats. */
        public stats?: (velobench.IStats|null);

        /** ServerFrame done. */
        public done?: (velobench.IDone|null);

        /** ServerFrame payload. */
        public payload?: ("delta"|"stats"|"done");

        /**
         * Creates a new ServerFrame instance using the specified properties.
         * @param [properties] Properties to set
         * @returns ServerFrame instance
         */
        public static create(properties?: velobench.IServerFrame): velobench.ServerFrame;

        /**
         * Encodes the specified ServerFrame message. Does not implicitly {@link velobench.ServerFrame.verify|verify} messages.
         * @param message ServerFrame message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: velobench.IServerFrame, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified ServerFrame message, length delimited. Does not implicitly {@link velobench.ServerFrame.verify|verify} messages.
         * @param message ServerFrame message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: velobench.IServerFrame, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes a ServerFrame message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns ServerFrame
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): velobench.ServerFrame;

        /**
         * Decodes a ServerFrame message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns ServerFrame
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): velobench.ServerFrame;

        /**
         * Verifies a ServerFrame message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates a ServerFrame message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns ServerFrame
         */
        public static fromObject(object: { [k: string]: any }): velobench.ServerFrame;

        /**
         * Creates a plain object from a ServerFrame message. Also converts values to other types if specified.
         * @param message ServerFrame
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: velobench.ServerFrame, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this ServerFrame to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };

        /**
         * Gets the default type url for ServerFrame
         * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns The default type url
         */
        public static getTypeUrl(typeUrlPrefix?: string): string;
    }

    /** Properties of a Delta. */
    interface IDelta {

        /** Delta content */
        content?: (string|null);

        /** Delta reasoning */
        reasoning?: (string|null);
    }

    /** Represents a Delta. */
    class Delta implements IDelta {

        /**
         * Constructs a new Delta.
         * @param [properties] Properties to set
         */
        constructor(properties?: velobench.IDelta);

        /** Delta content. */
        public content: string;

        /** Delta reasoning. */
        public reasoning: string;

        /**
         * Creates a new Delta instance using the specified properties.
         * @param [properties] Properties to set
         * @returns Delta instance
         */
        public static create(properties?: velobench.IDelta): velobench.Delta;

        /**
         * Encodes the specified Delta message. Does not implicitly {@link velobench.Delta.verify|verify} messages.
         * @param message Delta message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: velobench.IDelta, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified Delta message, length delimited. Does not implicitly {@link velobench.Delta.verify|verify} messages.
         * @param message Delta message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: velobench.IDelta, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes a Delta message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns Delta
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): velobench.Delta;

        /**
         * Decodes a Delta message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns Delta
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): velobench.Delta;

        /**
         * Verifies a Delta message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates a Delta message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns Delta
         */
        public static fromObject(object: { [k: string]: any }): velobench.Delta;

        /**
         * Creates a plain object from a Delta message. Also converts values to other types if specified.
         * @param message Delta
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: velobench.Delta, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this Delta to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };

        /**
         * Gets the default type url for Delta
         * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns The default type url
         */
        public static getTypeUrl(typeUrlPrefix?: string): string;
    }

    /** Properties of a DecodePoint. */
    interface IDecodePoint {

        /** DecodePoint tMs */
        tMs?: (number|null);

        /** DecodePoint tokS */
        tokS?: (number|null);

        /** DecodePoint kind */
        kind?: (string|null);

        /** DecodePoint regime */
        regime?: (string|null);
    }

    /** Represents a DecodePoint. */
    class DecodePoint implements IDecodePoint {

        /**
         * Constructs a new DecodePoint.
         * @param [properties] Properties to set
         */
        constructor(properties?: velobench.IDecodePoint);

        /** DecodePoint tMs. */
        public tMs: number;

        /** DecodePoint tokS. */
        public tokS: number;

        /** DecodePoint kind. */
        public kind: string;

        /** DecodePoint regime. */
        public regime: string;

        /**
         * Creates a new DecodePoint instance using the specified properties.
         * @param [properties] Properties to set
         * @returns DecodePoint instance
         */
        public static create(properties?: velobench.IDecodePoint): velobench.DecodePoint;

        /**
         * Encodes the specified DecodePoint message. Does not implicitly {@link velobench.DecodePoint.verify|verify} messages.
         * @param message DecodePoint message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: velobench.IDecodePoint, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified DecodePoint message, length delimited. Does not implicitly {@link velobench.DecodePoint.verify|verify} messages.
         * @param message DecodePoint message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: velobench.IDecodePoint, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes a DecodePoint message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns DecodePoint
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): velobench.DecodePoint;

        /**
         * Decodes a DecodePoint message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns DecodePoint
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): velobench.DecodePoint;

        /**
         * Verifies a DecodePoint message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates a DecodePoint message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns DecodePoint
         */
        public static fromObject(object: { [k: string]: any }): velobench.DecodePoint;

        /**
         * Creates a plain object from a DecodePoint message. Also converts values to other types if specified.
         * @param message DecodePoint
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: velobench.DecodePoint, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this DecodePoint to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };

        /**
         * Gets the default type url for DecodePoint
         * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns The default type url
         */
        public static getTypeUrl(typeUrlPrefix?: string): string;
    }

    /** Properties of an AcceptancePoint. */
    interface IAcceptancePoint {

        /** AcceptancePoint t */
        t?: (number|null);

        /** AcceptancePoint rate */
        rate?: (number|null);
    }

    /** Represents an AcceptancePoint. */
    class AcceptancePoint implements IAcceptancePoint {

        /**
         * Constructs a new AcceptancePoint.
         * @param [properties] Properties to set
         */
        constructor(properties?: velobench.IAcceptancePoint);

        /** AcceptancePoint t. */
        public t: number;

        /** AcceptancePoint rate. */
        public rate: number;

        /**
         * Creates a new AcceptancePoint instance using the specified properties.
         * @param [properties] Properties to set
         * @returns AcceptancePoint instance
         */
        public static create(properties?: velobench.IAcceptancePoint): velobench.AcceptancePoint;

        /**
         * Encodes the specified AcceptancePoint message. Does not implicitly {@link velobench.AcceptancePoint.verify|verify} messages.
         * @param message AcceptancePoint message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: velobench.IAcceptancePoint, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified AcceptancePoint message, length delimited. Does not implicitly {@link velobench.AcceptancePoint.verify|verify} messages.
         * @param message AcceptancePoint message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: velobench.IAcceptancePoint, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes an AcceptancePoint message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns AcceptancePoint
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): velobench.AcceptancePoint;

        /**
         * Decodes an AcceptancePoint message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns AcceptancePoint
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): velobench.AcceptancePoint;

        /**
         * Verifies an AcceptancePoint message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates an AcceptancePoint message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns AcceptancePoint
         */
        public static fromObject(object: { [k: string]: any }): velobench.AcceptancePoint;

        /**
         * Creates a plain object from an AcceptancePoint message. Also converts values to other types if specified.
         * @param message AcceptancePoint
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: velobench.AcceptancePoint, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this AcceptancePoint to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };

        /**
         * Gets the default type url for AcceptancePoint
         * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns The default type url
         */
        public static getTypeUrl(typeUrlPrefix?: string): string;
    }

    /** Properties of a SpecDepthPoint. */
    interface ISpecDepthPoint {

        /** SpecDepthPoint depth */
        depth?: (number|null);

        /** SpecDepthPoint count */
        count?: (number|null);
    }

    /** Represents a SpecDepthPoint. */
    class SpecDepthPoint implements ISpecDepthPoint {

        /**
         * Constructs a new SpecDepthPoint.
         * @param [properties] Properties to set
         */
        constructor(properties?: velobench.ISpecDepthPoint);

        /** SpecDepthPoint depth. */
        public depth: number;

        /** SpecDepthPoint count. */
        public count: number;

        /**
         * Creates a new SpecDepthPoint instance using the specified properties.
         * @param [properties] Properties to set
         * @returns SpecDepthPoint instance
         */
        public static create(properties?: velobench.ISpecDepthPoint): velobench.SpecDepthPoint;

        /**
         * Encodes the specified SpecDepthPoint message. Does not implicitly {@link velobench.SpecDepthPoint.verify|verify} messages.
         * @param message SpecDepthPoint message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: velobench.ISpecDepthPoint, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified SpecDepthPoint message, length delimited. Does not implicitly {@link velobench.SpecDepthPoint.verify|verify} messages.
         * @param message SpecDepthPoint message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: velobench.ISpecDepthPoint, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes a SpecDepthPoint message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns SpecDepthPoint
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): velobench.SpecDepthPoint;

        /**
         * Decodes a SpecDepthPoint message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns SpecDepthPoint
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): velobench.SpecDepthPoint;

        /**
         * Verifies a SpecDepthPoint message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates a SpecDepthPoint message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns SpecDepthPoint
         */
        public static fromObject(object: { [k: string]: any }): velobench.SpecDepthPoint;

        /**
         * Creates a plain object from a SpecDepthPoint message. Also converts values to other types if specified.
         * @param message SpecDepthPoint
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: velobench.SpecDepthPoint, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this SpecDepthPoint to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };

        /**
         * Gets the default type url for SpecDepthPoint
         * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns The default type url
         */
        public static getTypeUrl(typeUrlPrefix?: string): string;
    }

    /** Properties of a Cluster. */
    interface ICluster {

        /** Cluster mean */
        mean?: (number|null);

        /** Cluster count */
        count?: (number|null);

        /** Cluster std */
        std?: (number|null);

        /** Cluster min */
        min?: (number|null);

        /** Cluster max */
        max?: (number|null);
    }

    /** Represents a Cluster. */
    class Cluster implements ICluster {

        /**
         * Constructs a new Cluster.
         * @param [properties] Properties to set
         */
        constructor(properties?: velobench.ICluster);

        /** Cluster mean. */
        public mean: number;

        /** Cluster count. */
        public count: number;

        /** Cluster std. */
        public std: number;

        /** Cluster min. */
        public min: number;

        /** Cluster max. */
        public max: number;

        /**
         * Creates a new Cluster instance using the specified properties.
         * @param [properties] Properties to set
         * @returns Cluster instance
         */
        public static create(properties?: velobench.ICluster): velobench.Cluster;

        /**
         * Encodes the specified Cluster message. Does not implicitly {@link velobench.Cluster.verify|verify} messages.
         * @param message Cluster message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: velobench.ICluster, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified Cluster message, length delimited. Does not implicitly {@link velobench.Cluster.verify|verify} messages.
         * @param message Cluster message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: velobench.ICluster, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes a Cluster message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns Cluster
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): velobench.Cluster;

        /**
         * Decodes a Cluster message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns Cluster
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): velobench.Cluster;

        /**
         * Verifies a Cluster message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates a Cluster message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns Cluster
         */
        public static fromObject(object: { [k: string]: any }): velobench.Cluster;

        /**
         * Creates a plain object from a Cluster message. Also converts values to other types if specified.
         * @param message Cluster
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: velobench.Cluster, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this Cluster to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };

        /**
         * Gets the default type url for Cluster
         * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns The default type url
         */
        public static getTypeUrl(typeUrlPrefix?: string): string;
    }

    /** Properties of a ClusterResult. */
    interface IClusterResult {

        /** ClusterResult bimodal */
        bimodal?: (boolean|null);

        /** ClusterResult split */
        split?: (number|null);

        /** ClusterResult eta */
        eta?: (number|null);

        /** ClusterResult clusters */
        clusters?: (velobench.ICluster[]|null);

        /** ClusterResult total */
        total?: (number|null);
    }

    /** Represents a ClusterResult. */
    class ClusterResult implements IClusterResult {

        /**
         * Constructs a new ClusterResult.
         * @param [properties] Properties to set
         */
        constructor(properties?: velobench.IClusterResult);

        /** ClusterResult bimodal. */
        public bimodal: boolean;

        /** ClusterResult split. */
        public split: number;

        /** ClusterResult eta. */
        public eta: number;

        /** ClusterResult clusters. */
        public clusters: velobench.ICluster[];

        /** ClusterResult total. */
        public total: number;

        /**
         * Creates a new ClusterResult instance using the specified properties.
         * @param [properties] Properties to set
         * @returns ClusterResult instance
         */
        public static create(properties?: velobench.IClusterResult): velobench.ClusterResult;

        /**
         * Encodes the specified ClusterResult message. Does not implicitly {@link velobench.ClusterResult.verify|verify} messages.
         * @param message ClusterResult message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: velobench.IClusterResult, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified ClusterResult message, length delimited. Does not implicitly {@link velobench.ClusterResult.verify|verify} messages.
         * @param message ClusterResult message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: velobench.IClusterResult, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes a ClusterResult message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns ClusterResult
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): velobench.ClusterResult;

        /**
         * Decodes a ClusterResult message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns ClusterResult
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): velobench.ClusterResult;

        /**
         * Verifies a ClusterResult message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates a ClusterResult message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns ClusterResult
         */
        public static fromObject(object: { [k: string]: any }): velobench.ClusterResult;

        /**
         * Creates a plain object from a ClusterResult message. Also converts values to other types if specified.
         * @param message ClusterResult
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: velobench.ClusterResult, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this ClusterResult to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };

        /**
         * Gets the default type url for ClusterResult
         * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns The default type url
         */
        public static getTypeUrl(typeUrlPrefix?: string): string;
    }

    /** Properties of a LiveStat. */
    interface ILiveStat {

        /** LiveStat tokS */
        tokS?: (number|null);

        /** LiveStat avg */
        avg?: (number|null);

        /** LiveStat min */
        min?: (number|null);

        /** LiveStat median */
        median?: (number|null);

        /** LiveStat max */
        max?: (number|null);

        /** LiveStat tokens */
        tokens?: (number|null);

        /** LiveStat ttftMs */
        ttftMs?: (number|null);

        /** LiveStat genMs */
        genMs?: (number|null);

        /** LiveStat reasoningTokens */
        reasoningTokens?: (number|null);

        /** LiveStat contentTokens */
        contentTokens?: (number|null);
    }

    /** Represents a LiveStat. */
    class LiveStat implements ILiveStat {

        /**
         * Constructs a new LiveStat.
         * @param [properties] Properties to set
         */
        constructor(properties?: velobench.ILiveStat);

        /** LiveStat tokS. */
        public tokS: number;

        /** LiveStat avg. */
        public avg: number;

        /** LiveStat min. */
        public min: number;

        /** LiveStat median. */
        public median: number;

        /** LiveStat max. */
        public max: number;

        /** LiveStat tokens. */
        public tokens: number;

        /** LiveStat ttftMs. */
        public ttftMs: number;

        /** LiveStat genMs. */
        public genMs: number;

        /** LiveStat reasoningTokens. */
        public reasoningTokens: number;

        /** LiveStat contentTokens. */
        public contentTokens: number;

        /**
         * Creates a new LiveStat instance using the specified properties.
         * @param [properties] Properties to set
         * @returns LiveStat instance
         */
        public static create(properties?: velobench.ILiveStat): velobench.LiveStat;

        /**
         * Encodes the specified LiveStat message. Does not implicitly {@link velobench.LiveStat.verify|verify} messages.
         * @param message LiveStat message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: velobench.ILiveStat, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified LiveStat message, length delimited. Does not implicitly {@link velobench.LiveStat.verify|verify} messages.
         * @param message LiveStat message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: velobench.ILiveStat, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes a LiveStat message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns LiveStat
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): velobench.LiveStat;

        /**
         * Decodes a LiveStat message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns LiveStat
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): velobench.LiveStat;

        /**
         * Verifies a LiveStat message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates a LiveStat message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns LiveStat
         */
        public static fromObject(object: { [k: string]: any }): velobench.LiveStat;

        /**
         * Creates a plain object from a LiveStat message. Also converts values to other types if specified.
         * @param message LiveStat
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: velobench.LiveStat, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this LiveStat to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };

        /**
         * Gets the default type url for LiveStat
         * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns The default type url
         */
        public static getTypeUrl(typeUrlPrefix?: string): string;
    }

    /** Properties of a Stats. */
    interface IStats {

        /** Stats decode */
        decode?: (velobench.IDecodePoint[]|null);

        /** Stats latencies */
        latencies?: (number[]|null);

        /** Stats live */
        live?: (velobench.ILiveStat|null);

        /** Stats clusters */
        clusters?: (velobench.IClusterResult|null);

        /** Stats acceptance */
        acceptance?: (velobench.IAcceptancePoint[]|null);

        /** Stats specDepth */
        specDepth?: (velobench.ISpecDepthPoint[]|null);

        /** Stats regimes */
        regimes?: (velobench.IRegime[]|null);

        /** Stats category */
        category?: (string|null);

        /** Stats histMax */
        histMax?: (number|null);
    }

    /** Represents a Stats. */
    class Stats implements IStats {

        /**
         * Constructs a new Stats.
         * @param [properties] Properties to set
         */
        constructor(properties?: velobench.IStats);

        /** Stats decode. */
        public decode: velobench.IDecodePoint[];

        /** Stats latencies. */
        public latencies: number[];

        /** Stats live. */
        public live?: (velobench.ILiveStat|null);

        /** Stats clusters. */
        public clusters?: (velobench.IClusterResult|null);

        /** Stats acceptance. */
        public acceptance: velobench.IAcceptancePoint[];

        /** Stats specDepth. */
        public specDepth: velobench.ISpecDepthPoint[];

        /** Stats regimes. */
        public regimes: velobench.IRegime[];

        /** Stats category. */
        public category: string;

        /** Stats histMax. */
        public histMax: number;

        /**
         * Creates a new Stats instance using the specified properties.
         * @param [properties] Properties to set
         * @returns Stats instance
         */
        public static create(properties?: velobench.IStats): velobench.Stats;

        /**
         * Encodes the specified Stats message. Does not implicitly {@link velobench.Stats.verify|verify} messages.
         * @param message Stats message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: velobench.IStats, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified Stats message, length delimited. Does not implicitly {@link velobench.Stats.verify|verify} messages.
         * @param message Stats message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: velobench.IStats, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes a Stats message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns Stats
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): velobench.Stats;

        /**
         * Decodes a Stats message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns Stats
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): velobench.Stats;

        /**
         * Verifies a Stats message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates a Stats message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns Stats
         */
        public static fromObject(object: { [k: string]: any }): velobench.Stats;

        /**
         * Creates a plain object from a Stats message. Also converts values to other types if specified.
         * @param message Stats
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: velobench.Stats, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this Stats to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };

        /**
         * Gets the default type url for Stats
         * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns The default type url
         */
        public static getTypeUrl(typeUrlPrefix?: string): string;
    }

    /** Properties of a Regime. */
    interface IRegime {

        /** Regime category */
        category?: (string|null);

        /** Regime tokenCount */
        tokenCount?: (number|null);

        /** Regime avgTokS */
        avgTokS?: (number|null);

        /** Regime minTokS */
        minTokS?: (number|null);

        /** Regime medianTokS */
        medianTokS?: (number|null);

        /** Regime maxTokS */
        maxTokS?: (number|null);

        /** Regime samples */
        samples?: (velobench.IDecodePoint[]|null);
    }

    /** Represents a Regime. */
    class Regime implements IRegime {

        /**
         * Constructs a new Regime.
         * @param [properties] Properties to set
         */
        constructor(properties?: velobench.IRegime);

        /** Regime category. */
        public category: string;

        /** Regime tokenCount. */
        public tokenCount: number;

        /** Regime avgTokS. */
        public avgTokS: number;

        /** Regime minTokS. */
        public minTokS: number;

        /** Regime medianTokS. */
        public medianTokS: number;

        /** Regime maxTokS. */
        public maxTokS: number;

        /** Regime samples. */
        public samples: velobench.IDecodePoint[];

        /**
         * Creates a new Regime instance using the specified properties.
         * @param [properties] Properties to set
         * @returns Regime instance
         */
        public static create(properties?: velobench.IRegime): velobench.Regime;

        /**
         * Encodes the specified Regime message. Does not implicitly {@link velobench.Regime.verify|verify} messages.
         * @param message Regime message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: velobench.IRegime, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified Regime message, length delimited. Does not implicitly {@link velobench.Regime.verify|verify} messages.
         * @param message Regime message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: velobench.IRegime, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes a Regime message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns Regime
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): velobench.Regime;

        /**
         * Decodes a Regime message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns Regime
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): velobench.Regime;

        /**
         * Verifies a Regime message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates a Regime message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns Regime
         */
        public static fromObject(object: { [k: string]: any }): velobench.Regime;

        /**
         * Creates a plain object from a Regime message. Also converts values to other types if specified.
         * @param message Regime
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: velobench.Regime, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this Regime to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };

        /**
         * Gets the default type url for Regime
         * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns The default type url
         */
        public static getTypeUrl(typeUrlPrefix?: string): string;
    }

    /** Properties of a Done. */
    interface IDone {

        /** Done totalMs */
        totalMs?: (number|null);

        /** Done decodeMs */
        decodeMs?: (number|null);

        /** Done ttftMs */
        ttftMs?: (number|null);

        /** Done promptTokens */
        promptTokens?: (number|null);

        /** Done completionTokens */
        completionTokens?: (number|null);

        /** Done finalTokS */
        finalTokS?: (number|null);

        /** Done contentTokens */
        contentTokens?: (number|null);

        /** Done reasoningTokens */
        reasoningTokens?: (number|null);

        /** Done meta */
        meta?: (string|null);

        /** Done error */
        error?: (string|null);
    }

    /** Represents a Done. */
    class Done implements IDone {

        /**
         * Constructs a new Done.
         * @param [properties] Properties to set
         */
        constructor(properties?: velobench.IDone);

        /** Done totalMs. */
        public totalMs: number;

        /** Done decodeMs. */
        public decodeMs: number;

        /** Done ttftMs. */
        public ttftMs: number;

        /** Done promptTokens. */
        public promptTokens: number;

        /** Done completionTokens. */
        public completionTokens: number;

        /** Done finalTokS. */
        public finalTokS: number;

        /** Done contentTokens. */
        public contentTokens: number;

        /** Done reasoningTokens. */
        public reasoningTokens: number;

        /** Done meta. */
        public meta: string;

        /** Done error. */
        public error: string;

        /**
         * Creates a new Done instance using the specified properties.
         * @param [properties] Properties to set
         * @returns Done instance
         */
        public static create(properties?: velobench.IDone): velobench.Done;

        /**
         * Encodes the specified Done message. Does not implicitly {@link velobench.Done.verify|verify} messages.
         * @param message Done message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encode(message: velobench.IDone, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Encodes the specified Done message, length delimited. Does not implicitly {@link velobench.Done.verify|verify} messages.
         * @param message Done message or plain object to encode
         * @param [writer] Writer to encode to
         * @returns Writer
         */
        public static encodeDelimited(message: velobench.IDone, writer?: $protobuf.Writer): $protobuf.Writer;

        /**
         * Decodes a Done message from the specified reader or buffer.
         * @param reader Reader or buffer to decode from
         * @param [length] Message length if known beforehand
         * @returns Done
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decode(reader: ($protobuf.Reader|Uint8Array), length?: number): velobench.Done;

        /**
         * Decodes a Done message from the specified reader or buffer, length delimited.
         * @param reader Reader or buffer to decode from
         * @returns Done
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        public static decodeDelimited(reader: ($protobuf.Reader|Uint8Array)): velobench.Done;

        /**
         * Verifies a Done message.
         * @param message Plain object to verify
         * @returns `null` if valid, otherwise the reason why it is not
         */
        public static verify(message: { [k: string]: any }): (string|null);

        /**
         * Creates a Done message from a plain object. Also converts values to their respective internal types.
         * @param object Plain object
         * @returns Done
         */
        public static fromObject(object: { [k: string]: any }): velobench.Done;

        /**
         * Creates a plain object from a Done message. Also converts values to other types if specified.
         * @param message Done
         * @param [options] Conversion options
         * @returns Plain object
         */
        public static toObject(message: velobench.Done, options?: $protobuf.IConversionOptions): { [k: string]: any };

        /**
         * Converts this Done to JSON.
         * @returns JSON object
         */
        public toJSON(): { [k: string]: any };

        /**
         * Gets the default type url for Done
         * @param [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns The default type url
         */
        public static getTypeUrl(typeUrlPrefix?: string): string;
    }
}
