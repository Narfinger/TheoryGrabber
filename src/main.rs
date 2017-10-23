extern crate rss;
extern crate url;

use rss::Channel;
use url::Url;

static ARXIV: &'static str = "http://arxiv.org/rss/cs.CC";
static ECCC: &'static str = "http://eccc.hpi-web.de/feeds/reports/";

struct Paper {
    title: String,
    description: String,
    link: url::Url,
}

fn main() {
    let channel = Channel::from_url(ARXIV).unwrap();

    let papers = channel
        .items()
        .into_iter()
        .map(|i| {
            let title = i.title().unwrap();
            let description = i.description().unwrap();
            let link = i.link().unwrap();

            Paper {
                title: title.to_string(),
                description: description.to_string(),
                link: Url::parse(link).unwrap(),
            }
        });

}
