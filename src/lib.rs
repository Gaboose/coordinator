extern crate pest;
#[macro_use]
extern crate pest_derive;

use pest::Parser;
use pest::error::Error;
use fraction::Fraction;

#[derive(Parser)]
#[grammar = "fuko.pest"]
struct FukoParser;

fn read_all() -> Vec<u8> {
    let buf: [u8; 1024] = [0; 1024];
    let mut res: Vec<u8> = [].to_vec();

    loop {
        let n;
        unsafe {
            n = read(&buf[0], buf.len());
        }

        if n == 0 {
            return res
        }

        res.extend_from_slice(&buf[0 .. n]);
    }
}

fn write_bts(bts: &[u8]) {
    unsafe {
        write(&bts[0], bts.len());
    }
}

fn write_err_bts(bts: &[u8]) {
    unsafe {
        write_err(&bts[0], bts.len());
    }
}

#[derive(Debug)]
struct Source {
    amount: Fraction,
    to: Or,
}

#[derive(Debug)]
struct Date {
    year: u32,
    month: u32,
}

impl Date {
    fn from(s: &str) -> Date {
        let mut split = s.split("-");

        Date {
            year: split.next().unwrap().parse::<u32>().unwrap(),
            month: split.next().unwrap().parse::<u32>().unwrap(),
        }
    }

    fn after(&self, o: &Date) -> bool {
        self.year > o.year || (self.year == o.year && self.month > o.month)
    }

    fn to_string(&self) -> String {
        format!("{}-{:0>2}",self.year, self.month)
    }
}

#[derive(Debug)]
struct Sink {
    capacity: Fraction,
    balance: Fraction,
    add_amount: Fraction,
    date: Option<Date>,
    ask_for_balance: bool,
}

impl Sink {
    fn amount(&self) -> Fraction {
        self.balance + self.add_amount
    }
}

#[derive(Debug)]
struct Graph {
    sources: Vec<Source>,
    sinks: Vec<Sink>,
    unaries: Vec<UnaryCap>,
}

impl Graph {
    fn next_sinks(&self, or: &Or, amount_to_send: Fraction) -> (Vec<usize>, Vec<(usize, usize)>, Fraction) {
        let mut sinks = vec!();
        let mut unaries = vec!();
        let mut limited_amount = Fraction::infinity();

        for and in or {
            // let and_to_send = amount_to_send / Fraction::from(and.len());

            for ui in and {
                let unary = &self.unaries[*ui];
                if unary.amount >= unary.capacity {
                    // this expression node is already full, skip
                    continue
                }

                // let mut unary_to_send = and_to_send;
                let mut unary_to_send = amount_to_send;
                if unary.capacity - unary.amount < unary_to_send {
                    // expression node capacity limits step amount, modify returned amount
                    unary_to_send = unary.capacity - unary.amount;
                    if unary_to_send < limited_amount {
                        limited_amount = unary_to_send;
                        // write_err_bts(format!("DEBUG1 set limited_amount {} {} {}\n", limited_amount, unary.capacity, unary.amount).as_bytes());
                    }
                };
                
                let utpl = match &unary.unary {
                    Unary::Val(i) => {
                        if self.sinks[*i].capacity <= self.sinks[*i].amount() {
                            // sink is full, move on
                            continue
                        }

                        sinks.push(*i);

                        (*ui, 1)
                    }
                    Unary::Expr(e) => {
                        
                        let (mut more_sinks, mut more_unaries, new_limited_amount) = self.next_sinks(e, unary_to_send);
                        if more_sinks.len() == 0 {
                            continue
                        }

                        
                        // let new_limited_amount = a;
                        if new_limited_amount < limited_amount {
                            // capacity reached somewhere downstream, modify returned amount
                            limited_amount = new_limited_amount;
                            // write_err_bts(format!("DEBUG2 set limited_amount {}\n", limited_amount).as_bytes());
                        }

                        // write_err_bts(format!("DEBUG more_sinks {:?} {} {}\n", more_sinks, *ui, more_sinks.len()).as_bytes());
                        let utpl = (*ui, more_sinks.len());
                        unaries.append(&mut more_unaries);
                        sinks.append(&mut more_sinks);
                        // unaries.push((*ui, more_sinks.len()));
                        // write_err_bts(format!("DEBUG2\n").as_bytes());
                        
                        utpl
                    }
                };

                if unary.capacity < Fraction::infinity() {
                    unaries.push(utpl);
                }
            }

            if sinks.len() > 0 {
                break
            }
        };

        // let n = Fraction::from(sinks.len());

        (sinks, unaries, limited_amount)
    }

    fn next_step(&self) -> Step {
        // write_err_bts("DEBUG3\n".as_bytes());
        let mut res = Step{
            src_to_snks: vec!(),
            src_to_unaries: vec!(),
            amount: Fraction::infinity(),
            stop_reason: StopReason::NodeFull,
        };

        let mut fracs: Vec<Fraction> = self.sinks.iter().map(|_| {
            Fraction::from(0 as i32)
        }).collect();

        for (src_id, src) in self.sources.iter().enumerate() {
            if src.amount == Fraction::from(0 as i32) {
                continue
            }

            if src.amount < res.amount {
                res.amount = src.amount;
                res.stop_reason = StopReason::SourceDepleted;
            }

            let (sinks, unaries, limited_amount_sink) = self.next_sinks(&src.to, src.amount);
            // let limited_amount = amount_per_sink * Fraction::from(sinks.len());
            // write_err_bts(format!("amount_per_sink {:?}", amount_per_sink).as_bytes());
            // write_err_bts(format!("limited_amount_sink {:?}", limited_amount_sink).as_bytes());

            if sinks.len() == 0 {
                continue
            }

            let limited_amount = limited_amount_sink * Fraction::from(sinks.len());
            if limited_amount < res.amount {
                res.amount = limited_amount;
                res.stop_reason = StopReason::NodeFull;
            }

            for i in &sinks {
                fracs[*i] += Fraction::from(1 as i32) / Fraction::from(sinks.len());
            }
            // write_err_bts(format!("fracs {:?}", fracs).as_bytes());

            res.src_to_snks.push((src_id, sinks));
            res.src_to_unaries.push((src_id, unaries));
        }

        // write_err_bts("DEBUG4\n".as_bytes());

        for (i, frac) in fracs.iter().enumerate() {
            // write_err_bts(format!("DEBUG5 {} {} {} {}\n", i, frac, self.sinks[i].capacity, self.sinks[i].amount).as_bytes());
            let c = (self.sinks[i].capacity - self.sinks[i].amount()) / *frac;
            // write_err_bts(format!("DEBUG5.1\n").as_bytes());
            if c > Fraction::from(0 as i32) && c < res.amount {
                // write_err_bts(format!("DEBUG5.2\n").as_bytes());
                res.amount = c;
                res.stop_reason = StopReason::SinkFull;
            }
            // write_err_bts(format!("DEBUG5.3\n").as_bytes());
        }

        // write_err_bts("DEBUG6\n".as_bytes());

        res
    }

    fn apply_step(&mut self, s: &Step) {
        for (src_id, sink_ids) in &s.src_to_snks {
            self.sources[*src_id].amount -= s.amount;

            let per_sink = s.amount / Fraction::from(sink_ids.len());
            for i in sink_ids {
                self.sinks[*i].add_amount += per_sink;
            }

            for (src_id2, uns) in &s.src_to_unaries {
                if src_id == src_id2 {
                    for (u_id, mul) in uns {
                        // write_err_bts(format!("DEBUG3 adding unary amount {} {} {}\n", *u_id, per_sink, *mul).as_bytes());
                        self.unaries[*u_id].amount += per_sink * Fraction::from(*mul);
                    }
                    break;
                }
            }
            // write_err_bts("DEBUG2\n".as_bytes());
        }
    }
}

#[derive(Debug)]
enum StopReason {
    NodeFull,
    SourceDepleted,
    SinkFull,
}

#[derive(Debug)]
struct Step {
    src_to_snks: Vec<(usize, Vec<usize>)>,
    src_to_unaries: Vec<(usize, Vec<(usize, usize)>)>,
    amount: Fraction,
    stop_reason: StopReason,
}

#[derive(Debug)]
struct Transfer {
    from: usize,
    to: Vec<(usize, Fraction)>,
}

#[derive(Debug)]
struct Book {
    transfers: Vec<Transfer>
}

impl Book {
    fn get_transfer_or_insert(&mut self, from: usize) -> usize {
        for (i, t) in self.transfers.iter().enumerate() {
            if t.from == from {
                return i
            }
        }

        self.transfers.push(Transfer{
            from: from,
            to: vec!(),
        });

        self.transfers.len()-1
    }

    fn get_fraction_or_insert(&mut self, from: usize, to: usize) -> &mut Fraction {
        let i = self.get_transfer_or_insert(from);
        let to_vec = &mut self.transfers[i].to;
        let n = to_vec.len();

        for (j, (k, _)) in to_vec.iter().enumerate() {
            if *k == to {
                return &mut to_vec[j].1;
            }
        };

        to_vec.push((to, Fraction::from(0 as i32)));

        return &mut to_vec[n].1
    }

    fn apply_step(&mut self, s: &Step) {
        for (src_id, sink_ids) in &s.src_to_snks {
            let per_sink = s.amount / Fraction::from(sink_ids.len());
            for sink_id in sink_ids {
                let f = self.get_fraction_or_insert(*src_id, *sink_id);
                *f += per_sink;
            }
        }
    }
}

#[derive(Debug)]
struct FukoValue {
    source_names: Vec<String>,
    sink_names: Vec<String>,
    graph: Graph,
    book: Book,
}

type Or = Vec<And>;
type And = Vec<usize>;

#[derive(Debug)]
enum Unary {
    Val(usize),
    Expr(Or),
}

#[derive(Debug)]
struct UnaryCap {
    unary: Unary,
    amount: Fraction,
    capacity: Fraction,
}

enum ParseError {
    Rule(Error<Rule>),
    String(String),
}

impl ParseError {
    fn to_string(&self) -> String { 
        match self {
            ParseError::Rule(e) => e.to_string(),
            ParseError::String(s) => s.to_owned(),
        }
    }
}

use pest::iterators::Pair;

impl FukoValue {
    fn set_sink(&mut self, ident: &str, sink: Sink) -> bool {
        let (_, ok) = get_or_insert(&mut self.sink_names, ident);
        if !ok {
            return false;
        }

        self.graph.sinks.push(sink);
        true
    }

    fn get_or_insert_sink(&mut self, ident: &str) -> usize {
        let (i, ok) = get_or_insert(&mut self.sink_names, ident);

        if ok {
            self.graph.sinks.push(Sink{
                capacity: Fraction::infinity(),
                balance: Fraction::from(0 as i32),
                add_amount: Fraction::from(0 as i32),
                date: None,
                ask_for_balance: false,
            })
        }

        i
    }

    fn get_or_insert_source(&mut self, ident: &str) -> usize {
        let (i, ok) = get_or_insert(&mut self.source_names, ident);

        if ok {
            self.graph.sources.push(Source{
                amount: Fraction::from(0),
                to: vec!(),
            })
        }

        return i;
    }

    fn parse_expr(&mut self, expr: Pair<Rule>) -> Or {
        let or_pair = expr.into_inner().next().unwrap();
        let mut or = vec!();

        for and_pair in or_pair.into_inner() {
            let mut and = vec!();

            for unary_pair in and_pair.into_inner() {
                let mut unary_inner = unary_pair.into_inner();
                let ident_or_expr = unary_inner.next().unwrap();

                let cap = match unary_inner.next() {
                    Some(v) =>  Fraction::from(
                        v.into_inner().next().unwrap().as_str()
                         .parse::<f64>().unwrap()
                    ),
                    None => Fraction::infinity()
                };

                let uc = match ident_or_expr.as_rule() {
                    Rule::identifier => {
                        let s = ident_or_expr.as_str();
                        // and.push(UnaryCap{
                        //     unary: Unary::Val(self.get_or_insert_sink(s)),
                        //     capacity: cap,
                        // });
                        UnaryCap{
                            unary: Unary::Val(self.get_or_insert_sink(s)),
                            amount: Fraction::from(0 as i32),
                            capacity: cap,
                        }
                    }
                    Rule:: expr => {
                        // and.push(UnaryCap{
                        //     unary: Unary::Expr(self.parse_expr(ident_or_expr)),
                        //     capacity: cap,
                        // });
                        UnaryCap{
                            unary: Unary::Expr(self.parse_expr(ident_or_expr)),
                            amount: Fraction::from(0 as i32),
                            capacity: cap,
                        }
                    }
                    _ => unreachable!()
                };

                self.graph.unaries.push(uc);
                and.push(self.graph.unaries.len()-1);
            }

            or.push(and);
        }

        return or;
    }
}

fn get_or_insert(v: &mut Vec<String>, s: &str) -> (usize, bool) {
    let mut i = 0;
    loop {
        // write_err_bts("LOOP2".as_bytes());
        if i == v.len() {
            break;
        } else if v[i] == s {
            return (i, false)
        }
        i += 1;
    };

    v.push(s.to_owned());
    (v.len()-1, true)
}

fn parse_file<'a>(bts: &'a [u8]) -> Result<FukoValue, ParseError> {
    let file = match FukoParser::parse(Rule::file, std::str::from_utf8(&bts).unwrap()) {
        Ok(mut f) => f.next().unwrap(),
        Err(r) => return Err(ParseError::Rule(r)),
    };

    let mut res = FukoValue{
        source_names: vec!(),
        sink_names: vec!(),
        graph: Graph { sources: vec!(), sinks: vec!(), unaries: vec!() },
        book: Book{transfers: vec!()},
    };

    for stmt in file.into_inner() {
        match stmt.as_rule() {
            Rule::verb_statement => {
                let mut inner_rules = stmt.into_inner();
                let ident = inner_rules.next().unwrap().as_str();
                let verb = inner_rules.next().unwrap();
                let mut currency = inner_rules.next().unwrap().into_inner();

                match verb.as_rule() {
                    Rule::needs => {
                        let ask_for_balance = inner_rules.next();

                        let s = Sink{
                            capacity: Fraction::from(currency.next().unwrap().into_inner()
                                            .next().unwrap().as_str()
                                            .parse::<f64>().unwrap()),
                            balance: Fraction::from(0 as i32),
                            add_amount: Fraction::from(0 as i32),
                            date: None,
                            ask_for_balance: match ask_for_balance {
                                Some(_) => true,
                                None => false,
                            }
                        };
                        if !res.set_sink(ident, s) {
                            return Err(ParseError::String("recipient declared twice".to_owned()))
                        }
                    }
                    Rule::commits => {
                        let i = res.get_or_insert_source(ident);
                        let s = &mut res.graph.sources[i];
                        if s.amount != Fraction::from(0 as i32) {
                            return Err(ParseError::String("sender declared twice".to_owned()))
                        }
                        s.amount = Fraction::from(currency.next().unwrap().as_str()
                                    .parse::<f64>().unwrap());
                    }
                    _ => unreachable!()
                }
            }
            Rule::flow_statement => {
               let mut inner_rules = stmt.into_inner();

               let ident = inner_rules.next().unwrap().as_str();
               let expr = inner_rules.next().unwrap();

               let i = res.get_or_insert_source(ident);
               res.graph.sources[i].to = res.parse_expr(expr);
            }
            Rule::date_verb_statement => {
                let mut inner_rules = stmt.into_inner();
                let date = Date::from(inner_rules.next().unwrap().as_str());
                let ident = inner_rules.next().unwrap().as_str();
                let onetime_currency = inner_rules.next().unwrap().as_str().parse::<f64>().unwrap();

                let i = res.get_or_insert_sink(ident);
                if match &res.graph.sinks[i].date {
                    Some(d) => date.after(&d),
                    None => true,
                } {
                    res.graph.sinks[i].date = Some(date);
                    res.graph.sinks[i].balance = Fraction::from(onetime_currency);
                }
            }
            Rule::EOI => (),
            _ => unreachable!(),
        }
    }

    for (i, s) in res.graph.sinks.iter().enumerate() {
        if s.ask_for_balance && match s.date {Some(_) => false, None => true} {
            let name = &res.sink_names[i];
            return Err(ParseError::String(format!(
                "please provide a balance for {}\n\ne.g.:\nYYYY-MM {} had 10",
                name, name
            )));
        }
    }

    Ok(res)
}

fn main() {
    let bts = read_all();
    if bts.len() == 0 {
        return;
    }

   let mut fv = match parse_file(&bts) {
        Ok(v) => {
            // write_err_bts(format!("{:?}", v).as_bytes());
            v
        }
        Err(e) => {
            write_bts(e.to_string().as_bytes());
            return
        }
    };

    loop {
        // write_err_bts(format!("LOOP1 {:?}", fv.graph).as_bytes());
        let s = fv.graph.next_step();
        if s.src_to_snks.len() == 0 {
            break;
        }

        // write_err_bts("\n".as_bytes());
        write_err_bts(format!("{:?}\n", s).as_bytes());

        fv.graph.apply_step(&s);
        fv.book.apply_step(&s);
    }

    // print what sinks receive
    for (i, s) in fv.graph.sinks.iter().enumerate() {
        match &s.date {
            Some(d) => {
                write_bts(format!(
                    "{} <- {:.2} for {}\n",
                    fv.sink_names[i].as_str(),
                    s.add_amount,
                    d.to_string(),
                ).as_bytes());
            }
            None => {
                write_bts(format!(
                    "{} <- {:.2}\n",
                    fv.sink_names[i].as_str(),
                    s.add_amount,
                ).as_bytes());
            }
        }
    }

    write_bts("\n".as_bytes());

    // print transfers
    for tr in fv.book.transfers {
        for (snk_id, amount) in tr.to {
            match &fv.graph.sinks[snk_id].date {
                Some(d) => {
                    write_bts(format!(
                        "{} -> {}: {:.2} for {}\n",
                        fv.source_names[tr.from].as_str(),
                        fv.sink_names[snk_id].as_str(),
                        amount,
                        d.to_string(),
                    ).as_bytes());
                }
                None => {
                    write_bts(format!(
                        "{} -> {}: {:.2}\n",
                        fv.source_names[tr.from].as_str(),
                        fv.sink_names[snk_id].as_str(),
                        amount
                    ).as_bytes());
                }
            }
        }
        write_bts("\n".as_bytes());
    }
}

extern "C" {
    fn read(p: *const u8, l: usize) -> usize;
    fn write(p: *const u8, l: usize);
    fn write_err(p: *const u8, l: usize);
}

#[no_mangle]
pub extern "C" fn _start() {
    main();
}