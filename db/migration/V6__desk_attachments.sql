/*
  Attachments of desk tickets (ADR-0009). The bytes live in an S3 bucket (RustFS locally); the record is the event
  TicketAttachmentAdded in the ticket's stream, and crm.ticket_attachments is its read model.
  What may be attached is data: the type comes from the extension, never from what the browser claims. SVG and HTML
  are missing on purpose - shown inline they can run scripts.
*/
CREATE TABLE crm.desk_attachment_types
(
    extension    text    NOT NULL,
    content_type text    NOT NULL,
    -- Raster images and PDF open in the browser; everything else is downloaded.
    is_inline    boolean NOT NULL,
    CONSTRAINT pk_desk_attachment_types PRIMARY KEY (extension)
);

INSERT INTO crm.desk_attachment_types (extension, content_type, is_inline)
VALUES ('.png', 'image/png', true),
       ('.jpg', 'image/jpeg', true),
       ('.jpeg', 'image/jpeg', true),
       ('.gif', 'image/gif', true),
       ('.webp', 'image/webp', true),
       ('.pdf', 'application/pdf', true),
       ('.txt', 'text/plain', false),
       ('.log', 'text/plain', false),
       ('.csv', 'text/csv', false),
       ('.docx', 'application/vnd.openxmlformats-officedocument.wordprocessingml.document', false),
       ('.xlsx', 'application/vnd.openxmlformats-officedocument.spreadsheetml.sheet', false),
       ('.pptx', 'application/vnd.openxmlformats-officedocument.presentationml.presentation', false),
       ('.odt', 'application/vnd.oasis.opendocument.text', false),
       ('.ods', 'application/vnd.oasis.opendocument.spreadsheet', false),
       ('.zip', 'application/zip', false),
       ('.mp4', 'video/mp4', false),
       ('.mov', 'video/quicktime', false);

CREATE TABLE crm.ticket_attachments
(
    attachment_id    uuid           NOT NULL,
    ticket_id        bigint         NOT NULL,
    file_name        text           NOT NULL,
    content_type     text           NOT NULL,
    size_bytes       bigint         NOT NULL,
    storage_key      text           NOT NULL,
    -- The agent's name, or the requester's on the public form (a snapshot, like the event).
    added_by         text           NOT NULL,
    added_by_user_id int            NULL,
    at               timestamptz(3) NOT NULL,
    CONSTRAINT pk_ticket_attachments PRIMARY KEY (attachment_id),
    CONSTRAINT uq_ticket_attachments_key UNIQUE (storage_key),
    CONSTRAINT fk_ticket_attachments_ticket FOREIGN KEY (ticket_id) REFERENCES crm.tickets (id),
    CONSTRAINT ck_ticket_attachments_size CHECK (size_bytes > 0),
    CONSTRAINT ck_ticket_attachments_name CHECK (length(file_name) BETWEEN 1 AND 200)
);

CREATE INDEX ix_ticket_attachments_ticket ON crm.ticket_attachments (ticket_id, at);
