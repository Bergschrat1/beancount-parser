use std::borrow::Borrow;
use std::collections::HashSet;
use std::fmt::{Display, Formatter};
use std::sync::Arc;

use nom::{
    branch::alt,
    bytes::complete::{tag, take_while},
    character::complete::satisfy,
    character::complete::{char as char_tag, space0, space1},
    combinator::{cut, iterator, map, opt, success, value},
    sequence::{delimited, preceded, separated_pair, terminated, tuple},
    Parser,
};

use crate::{
    account, account::Account, amount, amount::Amount, date, empty_line, end_of_line, metadata,
    string, Date, Decimal, IResult, Span,
};

/// A transaction
///
/// It notably contains a list of [`Posting`]
///
/// # Example
/// ```
/// # use beancount_parser_2::{DirectiveContent};
/// let input = r#"
/// 2022-05-22 * "Grocery store" "Grocery shopping" #food
///   Assets:Cash           -10 CHF
///   Expenses:Groceries
/// "#;
///
/// let beancount = beancount_parser_2::parse::<f64>(input).unwrap();
/// let DirectiveContent::Transaction(trx) = &beancount.directives[0].content else {
///   unreachable!("was not a transaction")
/// };
/// assert_eq!(trx.flag, Some('*'));
/// assert_eq!(trx.payee.as_deref(), Some("Grocery store"));
/// assert_eq!(trx.narration.as_deref(), Some("Grocery shopping"));
/// assert!(trx.tags.contains("food"));
/// assert_eq!(trx.postings.len(), 2);
/// ```
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct Transaction<D> {
    /// Transaction flag (`*` or `!` or `None` when using the `txn` keyword)
    pub flag: Option<char>,
    /// Payee (if present)
    pub payee: Option<String>,
    /// Narration (if present)
    pub narration: Option<String>,
    /// Set of tags
    pub tags: HashSet<Tag>,
    /// Set of links
    pub links: HashSet<Link>,
    /// Postings
    pub postings: Vec<Posting<D>>,
}

/// A transaction posting
///
/// # Example
/// ```
/// # use beancount_parser_2::{DirectiveContent, PostingPrice};
/// let input = r#"
/// 2022-05-22 * "Grocery shopping"
///   Assets:Cash           1 CHF {2 PLN} @ 3 EUR
///   Expenses:Groceries
/// "#;
///
/// let beancount = beancount_parser_2::parse::<f64>(input).unwrap();
/// let DirectiveContent::Transaction(trx) = &beancount.directives[0].content else {
///   unreachable!("was not a transaction")
/// };
/// let posting = &trx.postings[0];
/// assert_eq!(posting.account.as_str(), "Assets:Cash");
/// assert_eq!(posting.amount.as_ref().unwrap().value, 1.0);
/// assert_eq!(posting.amount.as_ref().unwrap().currency.as_str(), "CHF");
/// assert_eq!(posting.cost.as_ref().unwrap().amount.as_ref().unwrap().value, 2.0);
/// assert_eq!(posting.cost.as_ref().unwrap().amount.as_ref().unwrap().currency.as_str(), "PLN");
/// let Some(PostingPrice::Unit(price)) = &posting.price else {
///   unreachable!("no price");
/// };
/// assert_eq!(price.value, 3.0);
/// assert_eq!(price.currency.as_str(), "EUR");
/// ```
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct Posting<D> {
    /// Transaction flag (`*` or `!` or `None` when absent)
    pub flag: Option<char>,
    /// Account modified by the posting
    pub account: Account,
    /// Amount being added to the account
    pub amount: Option<Amount<D>>,
    /// Cost (content within `{` and `}`)
    pub cost: Option<Cost<D>>,
    /// Price (`@` or `@@`) syntax
    pub price: Option<PostingPrice<D>>,
    /// The metadata attached to the posting
    pub metadata: metadata::Map<D>,
}

/// Cost of a posting
///
/// It is the amount within `{` and `}`.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct Cost<D> {
    /// Cost basis of the posting
    pub amount: Option<Amount<D>>,
    /// The date of this cost basis
    pub date: Option<Date>,
}

/// Price of a posting
///
/// It is the amount following the `@` or `@@` symbols
#[derive(Debug, Clone)]
pub enum PostingPrice<D> {
    /// Unit cost (`@`)
    Unit(Amount<D>),
    /// Total cost (`@@`)
    Total(Amount<D>),
}

/// Transaction tag
///
/// # Example
/// ```
/// # use beancount_parser_2::{DirectiveContent};
/// let input = r#"
/// 2022-05-22 * "Grocery store" "Grocery shopping" #food
///   Assets:Cash           -10 CHF
///   Expenses:Groceries
/// "#;
///
/// let beancount = beancount_parser_2::parse::<f64>(input).unwrap();
/// let DirectiveContent::Transaction(trx) = &beancount.directives[0].content else {
///   unreachable!("was not a transaction")
/// };
/// assert!(trx.tags.contains("food"));
/// ```
#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Tag(Arc<str>);

impl Tag {
    /// Returns underlying string representation
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Display for Tag {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&self.0, f)
    }
}

impl AsRef<str> for Tag {
    fn as_ref(&self) -> &str {
        self.0.as_ref()
    }
}

impl Borrow<str> for Tag {
    fn borrow(&self) -> &str {
        self.0.borrow()
    }
}

/// Transaction link
///
/// # Example
/// ```
/// # use beancount_parser_2::{DirectiveContent};
/// let input = r#"
/// 2014-02-05 * "Invoice for January" ^invoice-pepe-studios-jan14
///    Income:Clients:PepeStudios           -8450.00 USD
///    Assets:AccountsReceivable
/// "#;
///
/// let beancount = beancount_parser_2::parse::<f64>(input).unwrap();
/// let DirectiveContent::Transaction(trx) = &beancount.directives[0].content else {
///   unreachable!("was not a transaction")
/// };
/// assert!(trx.links.contains("invoice-pepe-studios-jan14"));
/// ```
#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Link(Arc<str>);

impl Link {
    /// Returns underlying string representation
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Display for Link {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&self.0, f)
    }
}

impl AsRef<str> for Link {
    fn as_ref(&self) -> &str {
        self.0.as_ref()
    }
}

impl Borrow<str> for Link {
    fn borrow(&self) -> &str {
        self.0.borrow()
    }
}

#[allow(clippy::type_complexity)]
pub(crate) fn parse<D: Decimal>(
    input: Span<'_>,
) -> IResult<'_, (Transaction<D>, metadata::Map<D>)> {
    let (input, flag) = alt((map(flag, Some), value(None, tag("txn"))))(input)?;
    cut(do_parse(flag))(input)
}

fn flag(input: Span<'_>) -> IResult<'_, char> {
    satisfy(|c: char| !c.is_ascii_lowercase())(input)
}

fn do_parse<D: Decimal>(
    flag: Option<char>,
) -> impl Fn(Span<'_>) -> IResult<'_, (Transaction<D>, metadata::Map<D>)> {
    move |input| {
        let (input, payee_and_narration) = opt(preceded(space1, payee_and_narration))(input)?;
        let (input, (tags, links)) = tags_and_links(input)?;
        let (input, _) = end_of_line(input)?;
        let (input, metadata) = metadata::parse(input)?;
        let mut iter = iterator(input, alt((posting.map(Some), empty_line.map(|_| None))));
        let postings = iter.flatten().collect();
        let (input, _) = iter.finish()?;
        let narration = payee_and_narration.map(|(_, n)| n).map(ToOwned::to_owned);
        let payee = payee_and_narration
            .and_then(|(p, _)| p)
            .map(ToOwned::to_owned);
        Ok((
            input,
            (
                Transaction {
                    flag,
                    payee,
                    narration,
                    tags,
                    links,
                    postings,
                },
                metadata,
            ),
        ))
    }
}

pub(super) enum TagOrLink {
    Tag(Tag),
    Link(Link),
}

pub(super) fn parse_tag(input: Span<'_>) -> IResult<'_, Tag> {
    map(
        preceded(
            char_tag('#'),
            take_while(|c: char| c.is_alphanumeric() || c == '-' || c == '_'),
        ),
        |s: Span<'_>| Tag((*s.fragment()).into()),
    )(input)
}

pub(super) fn parse_link(input: Span<'_>) -> IResult<'_, Link> {
    map(
        preceded(
            char_tag('^'),
            take_while(|c: char| c.is_alphanumeric() || c == '-' || c == '_'),
        ),
        |s: Span<'_>| Link((*s.fragment()).into()),
    )(input)
}

pub(super) fn parse_tag_or_link(input: Span<'_>) -> IResult<'_, TagOrLink> {
    alt((
        map(parse_tag, TagOrLink::Tag),
        map(parse_link, TagOrLink::Link),
    ))(input)
}

fn tags_and_links(input: Span<'_>) -> IResult<'_, (HashSet<Tag>, HashSet<Link>)> {
    let mut tags_and_links_iter = iterator(input, preceded(space1, parse_tag_or_link));
    let (tags, links) = tags_and_links_iter.fold(
        (HashSet::new(), HashSet::new()),
        |(mut tags, mut links), x| {
            match x {
                TagOrLink::Tag(tag) => tags.insert(tag),
                TagOrLink::Link(link) => links.insert(link),
            };
            (tags, links)
        },
    );
    let (input, _) = tags_and_links_iter.finish()?;
    Ok((input, (tags, links)))
}

fn payee_and_narration(input: Span<'_>) -> IResult<'_, (Option<&str>, &str)> {
    let (input, s1) = string(input)?;
    let (input, s2) = opt(preceded(space1, string))(input)?;
    Ok((
        input,
        match s2 {
            Some(narration) => (Some(s1), narration),
            None => (None, s1),
        },
    ))
}

fn posting<D: Decimal>(input: Span<'_>) -> IResult<'_, Posting<D>> {
    let (input, _) = space1(input)?;
    let (input, flag) = opt(terminated(flag, space1))(input)?;
    let (input, account) = account::parse(input)?;
    let (input, amounts) = opt(tuple((
        preceded(space1, amount::parse),
        opt(preceded(space1, cost)),
        opt(preceded(
            space1,
            alt((
                map(
                    preceded(tuple((char_tag('@'), space1)), amount::parse),
                    PostingPrice::Unit,
                ),
                map(
                    preceded(tuple((tag("@@"), space1)), amount::parse),
                    PostingPrice::Total,
                ),
            )),
        )),
    )))(input)?;
    let (input, _) = end_of_line(input)?;
    let (input, metadata) = metadata::parse(input)?;
    let (amount, cost, price) = match amounts {
        Some((a, l, p)) => (Some(a), l, p),
        None => (None, None, None),
    };
    Ok((
        input,
        Posting {
            flag,
            account,
            amount,
            cost,
            price,
            metadata,
        },
    ))
}

fn cost<D: Decimal>(input: Span<'_>) -> IResult<'_, Cost<D>> {
    let (input, _) = terminated(char_tag('{'), space0)(input)?;
    let (input, (cost, date)) = alt((
        map(
            separated_pair(
                amount::parse,
                delimited(space0, char_tag(','), space0),
                date::parse,
            ),
            |(a, d)| (Some(a), Some(d)),
        ),
        map(
            separated_pair(
                date::parse,
                delimited(space0, char_tag(','), space0),
                amount::parse,
            ),
            |(d, a)| (Some(a), Some(d)),
        ),
        map(amount::parse, |a| (Some(a), None)),
        map(date::parse, |d| (None, Some(d))),
        map(success(true), |_| (None, None)),
    ))(input)?;
    let (input, _) = preceded(space0, char_tag('}'))(input)?;
    Ok((input, Cost { amount: cost, date }))
}
