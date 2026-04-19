export function PageHeader({
  eyebrow = "Operator surface",
  title,
  titleId,
  description,
  meta,
}: {
  eyebrow?: string;
  title: string;
  titleId?: string;
  description: string;
  meta?: string;
}) {
  return (
    <header className="page-header">
      <div className="page-header__eyebrow-row">
        <p className="page-header__eyebrow">{eyebrow}</p>
        {meta ? <p className="page-header__meta">{meta}</p> : null}
      </div>
      <div className="page-header__body">
        <h1 className="page-header__title" id={titleId}>
          {title}
        </h1>
        <p className="page-header__description">{description}</p>
      </div>
    </header>
  );
}
