use clap::{Parser, ValueEnum};
use std::process::Command;

#[derive(Parser)]
#[command(name = "gh-commit")]
#[command(bin_name = "gh-commit")]
#[clap(author, version, about, long_about = None)]
pub struct Args {
    #[arg(long,action = clap::ArgAction::SetTrue, default_value_t = false,help="项目初始化")]
    init: bool,

    #[arg(long = "feat", help = "添加新特性")]
    feature: Option<String>,

    #[arg(long = "fix", help = "修复Bug")]
    fix: Option<String>,

    #[arg(long = "docs", help = "仅仅修改文档")]
    docs: Option<String>,

    #[arg(long = "style", help = "仅仅修改了空格、格式缩进、逗号等")]
    style: Option<String>,

    #[arg(long = "refactor", help = "代码重构，没有加新功能或者修复bug")]
    refactor: Option<String>,

    #[arg(long = "perf", help = "优化相关，比如提升性能、体验")]
    perf: Option<String>,

    #[arg(long = "test", help = "单元测试的添加或修复")]
    test: Option<String>,

    #[arg(long = "chore", help = "改变构建流程、或者增加依赖库、工具等")]
    chore: Option<String>,

    #[arg(long = "revert", help = "回滚到上一个版本")]
    revert: Option<String>,

    #[arg(short = 'b', long, help = "Message Body")]
    body: Option<String>,

    #[arg(long = "pr", help = "With PR")]
    pr: Vec<i64>,

    #[arg(long = "closes", help = "With closes")]
    closes: Vec<i64>,

    #[arg(long = "breaks", help = "With breaks")]
    breaks: Vec<String>,

    #[arg(short='c', long,action = clap::ArgAction::SetTrue, default_value_t = false)]
    commit: bool,

    #[arg(long="hide",action = clap::ArgAction::SetTrue, default_value_t = false)]
    hide_icon: bool,

    #[arg(help = "Message Subject")]
    message: Option<Vec<String>>,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
pub enum CommitType {
    Init,     // 初始化
    Feature,  // 新增功能
    Fix,      // Bug修复
    Docs,     // 编辑文档
    Refactor, // 重构
    Style,    // 样式
    Perf,     // 性能优化
    Test,     // 单元测试的添加或修复
    Chore,    // 构建工具的修改
    Revert,   // 回滚
}

impl CommitType {
    fn icon(&self) -> &str {
        match self {
            CommitType::Init => "🎉",
            CommitType::Feature => "✨",
            CommitType::Fix => "🐞",
            CommitType::Docs => "📃",
            CommitType::Style => "🌈",
            CommitType::Refactor => "🦄",
            CommitType::Perf => "🚀",
            CommitType::Test => "🧪",
            CommitType::Chore => "🔧",
            CommitType::Revert => "🙃",
        }
    }

    fn name(&self) -> &str {
        match self {
            CommitType::Init => "Initializing",
            CommitType::Feature => "feat",
            CommitType::Fix => "fix",
            CommitType::Docs => "docs",
            CommitType::Style => "style",
            CommitType::Refactor => "refactor",
            CommitType::Perf => "perf",
            CommitType::Test => "test",
            CommitType::Chore => "chore",
            CommitType::Revert => "revert",
        }
    }
}

// [ICON] [TYPES]([SCOPES]): [SUBJECT] (#pr)
// <\n>
// [BODY]
// <\n>
// [FOOTER]

pub fn execute(args: &Args) {
    let mut header = String::new();
    if args.init {
        header.push_str(format!("{} Initializing", CommitType::Init.icon()).as_str());
    } else {
        let commit_types: Vec<(Option<String>, CommitType)> = vec![
            (args.feature.clone(), CommitType::Feature),
            (args.fix.clone(), CommitType::Fix),
            (args.docs.clone(), CommitType::Docs),
            (args.style.clone(), CommitType::Style),
            (args.refactor.clone(), CommitType::Refactor),
            (args.perf.clone(), CommitType::Perf),
            (args.test.clone(), CommitType::Test),
            (args.chore.clone(), CommitType::Chore),
            (args.revert.clone(), CommitType::Revert),
        ];
        for (action, ct) in commit_types {
            if let Some(scope) = action {
                if args.hide_icon {
                    header.push_str(format!("{}({})", ct.name(), scope).as_str());
                } else {
                    header.push_str(format!("{} {}({})", ct.icon(), ct.name(), scope).as_str());
                }
            }
        }

        if let Some(message) = args.message.clone() {
            header.push_str(format!(": {} ", message.join(" ")).as_str());
        }

        for pr in args.pr.clone() {
            header.push_str(format!("(#{})", pr).as_str());
        }
    }

    let body = if let Some(body) = args.body.clone() {
        body
    } else {
        "".to_string()
    };

    let mut footer: Vec<String> = vec![];
    let closes = args.closes.clone();
    if closes.len() > 0 {
        footer.push(format!(
            "Closes: {}",
            closes
                .iter()
                .map(|&num| format!("#{}", num))
                .collect::<Vec<String>>()
                .join(", ")
        ));
    }

    let breaks = args.breaks.clone();
    if breaks.len() > 0 {
        footer.push(format!("Breaks: {}", breaks.join(", ")));
    }

    let result = if footer.len() > 0 {
        if body.len() > 0 {
            format!("{}\n\n{}\n\n{}", header, body, footer.join("\n"))
        } else {
            format!("{}\n\n\n\n{}", header, footer.join("\n"))
        }
    } else if body.len() > 0 {
        format!("{}\n\n{}", header, body)
    } else {
        header
    };

    if args.commit {
        match Command::new("git")
            .args(["commit", "-m", result.as_str()])
            .output()
        {
            Ok(output) => {
                println!("{}", String::from_utf8_lossy(&output.stdout));
            }
            Err(e) => {
                println!("error: {}", e);
            }
        }
    } else {
        println!("{}", result);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_commit_execute() {
        execute(&Args::parse_from(&["--help"]));
    }

    #[test]
    fn test_print_commit_types() {
        let commit_types: Vec<CommitType> = vec![
            CommitType::Feature,
            CommitType::Refactor,
            CommitType::Perf,
            CommitType::Fix,
            CommitType::Test,
            CommitType::Chore,
            CommitType::Docs,
            CommitType::Style,
            CommitType::Revert,
        ];
        for ct in commit_types {
            println!("- {} {}", ct.icon(), ct.name(),);
        }
    }
}
