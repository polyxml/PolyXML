/**
 * PolyXML TypeScript Type Definitions
 */

export type FieldKind = 'attribute' | 'element' | 'text';

export type ScalarType =
  | 'string'
  | 'int'
  | 'float'
  | 'bool'
  | 'decimal'
  | 'xml_date'
  | 'xml_datetime'
  | 'any';

export interface FieldDefinition {
  name: string;
  xmlName: string;
  kind: FieldKind;
  scalarType?: ScalarType | null;
  namespace?: string | null;
  nestedSchema?: ModelSchema | null;
  isList?: boolean | null;
  required?: boolean | null;
}

export interface ModelSchema {
  name: string;
  xmlName?: string | null;
  namespace?: string | null;
  fields: FieldDefinition[];
  isAbstract?: boolean | null;
  variants?: ModelSchema[] | null;
}

export type PolyValue =
  | null
  | boolean
  | number
  | string
  | PolyValue[]
  | { [key: string]: PolyValue };

/**
 * Deserialize an XML string or Uint8Array into a JavaScript object based on schema.
 */
export function deserialize(
  xml: string | Uint8Array,
  schema: ModelSchema
): PolyValue;

/**
 * Serialize a JavaScript object into XML bytes based on schema.
 */
export function serialize(
  rootName: string,
  value: Record<string, any>,
  schema: ModelSchema,
  indent?: number | null,
  namespaces?: boolean | null,
  nsMap?: Record<string, string> | null
): Uint8Array;

/**
 * Return the PolyXML engine version.
 */
export function version(): string;
