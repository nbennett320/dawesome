//
// This file describes the application's document model and hierarchy.
//

interface BaseDocumentEntity {
  id: uuid
}

export interface Project extends BaseDocumentEntity {
  name: string
}

export interface TimelineTrack extends BaseDocumentEntity {
  name: string
  color: string
  entities: Entities
}

export type Entity = Sample | Midi
export type Entities = Record<uuid, Entity>

export interface Sample extends BaseDocumentEntity {
  name: string
  color: string
  assetId: uuid
}

export interface Midi extends BaseDocumentEntity {
  name: string
  color: string
  assetId: uuid
}

export interface Asset extends BaseDocumentEntity {
  path: string
}

export type uuid = string
