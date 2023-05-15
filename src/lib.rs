mod account;
mod currency;
mod date;
mod open;

pub use date::Date;
use nom::{
    branch::alt,
    bytes::complete::tag,
    character::complete::{char, line_ending, not_line_ending, space0, space1},
    combinator::{all_consuming, cut, eof, iterator, map, opt},
    sequence::preceded,
    Finish, Parser,
};

pub fn parse(input: &str) -> Result<BeancountFile<'_>, Error<'_>> {
    match all_consuming(beancount_file)(Span::new(input)).finish() {
        Ok((_, content)) => Ok(content),
        Err(nom::error::Error { input, .. }) => Err(Error(input)),
    }
}

#[derive(Debug)]
pub struct Error<'a>(Span<'a>);

#[derive(Debug)]
#[non_exhaustive]
pub struct BeancountFile<'a> {
    pub directives: Vec<Directive<'a>>,
}

#[derive(Debug)]
#[non_exhaustive]
pub struct Directive<'a> {
    pub date: Date,
    pub content: DirectiveContent<'a>,
}

#[derive(Debug)]
#[non_exhaustive]
pub enum DirectiveContent<'a> {
    Open(open::Open<'a>),
}

type Span<'a> = nom_locate::LocatedSpan<&'a str>;
type IResult<'a, O> = nom::IResult<Span<'a>, O>;

fn beancount_file(input: Span<'_>) -> IResult<'_, BeancountFile<'_>> {
    let mut iter = iterator(input, alt((directive.map(Some), line.map(|_| None))));
    let directives = iter.flatten().collect();
    let (input, _) = iter.finish()?;
    Ok((input, BeancountFile { directives }))
}

fn directive(input: Span<'_>) -> IResult<'_, Directive<'_>> {
    let (input, date) = date::parse(input)?;
    let (input, content) = cut(directive_content)(input)?;
    Ok((input, Directive { date, content }))
}

fn directive_content(input: Span<'_>) -> IResult<'_, DirectiveContent<'_>> {
    let (input, _) = space1(input)?;
    let (input, content) = alt((map(
        preceded(tag("open"), cut(preceded(space1, open::parse))),
        DirectiveContent::Open,
    ),))(input)?;
    let (input, _) = space0(input)?;
    let (input, _) = opt(comment)(input)?;
    let (input, _) = alt((line_ending, eof))(input)?;
    Ok((input, content))
}

fn comment(input: Span<'_>) -> IResult<'_, ()> {
    let (input, _) = char(';')(input)?;
    let (input, _) = not_line_ending(input)?;
    Ok((input, ()))
}

fn line(input: Span<'_>) -> IResult<'_, ()> {
    let (input, _) = not_line_ending(input)?;
    let (input, _) = line_ending(input)?;
    Ok((input, ()))
}
