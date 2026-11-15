//! Prompts: ready-made requests that tell the assistant which tools to call, in which order, with
//! which arguments. They name only tools and options of this server.

use serde_json::{Map, Value, json};

struct PromptDef {
    name: &'static str,
    description: &'static str,
    /// (name, description, required)
    args: &'static [(&'static str, &'static str, bool)],
}

const PROMPTS: &[PromptDef] = &[
    PromptDef {
        name: "plot_cv_files",
        description: "Import data files (CH Instruments voltammograms or other text data) and overlay them in one figure.",
        args: &[(
            "files",
            "The files, folders or patterns to import, for example run1.txt run2.txt or data/*.txt",
            true,
        )],
    },
    PromptDef {
        name: "integrate_peak",
        description: "Integrate a peak between two X values with a straight baseline and report the area.",
        args: &[
            ("column", "The Y column (short or long name)", true),
            ("from", "The lower X value of the peak", true),
            ("to", "The upper X value of the peak", true),
        ],
    },
    PromptDef {
        name: "fit_curve",
        description: "Fit a model to a column and report the parameters with their errors.",
        args: &[
            ("column", "The Y column (short or long name)", true),
            ("model", "linear, poly:ORDER, or a built-in function such as Gauss (see list_fit_models)", true),
        ],
    },
];

/// The `prompts/list` result.
pub fn list() -> Value {
    let prompts: Vec<Value> = PROMPTS
        .iter()
        .map(|p| {
            let args: Vec<Value> =
                p.args.iter().map(|(n, d, r)| json!({"name": n, "description": d, "required": r})).collect();
            json!({"name": p.name, "title": p.name.replace('_', " "), "description": p.description, "arguments": args})
        })
        .collect();
    json!({ "prompts": prompts })
}

/// A quoted JSON string, so a name with quotes or backslashes stays valid inside a tool call.
fn q(text: &str) -> String {
    Value::String(text.to_string()).to_string()
}

/// The `prompts/get` result, or the reason the request is invalid.
pub fn get(name: &str, args: &Map<String, Value>) -> Result<Value, String> {
    let def = PROMPTS.iter().find(|p| p.name == name).ok_or_else(|| format!("unknown prompt: {name}"))?;
    let mut values = Vec::new();
    for (arg, _, required) in def.args {
        match args.get(*arg) {
            Some(Value::String(s)) if !s.trim().is_empty() => values.push(s.trim().to_string()),
            Some(Value::Number(n)) => values.push(n.to_string()),
            _ if *required => return Err(format!("missing required argument '{arg}' for prompt {name}")),
            _ => values.push(String::new()),
        }
    }
    let text = match name {
        "plot_cv_files" => {
            let files: Vec<String> = values[0].split_whitespace().map(q).collect();
            let list = files.join(", ");
            format!(
                "Import these files with Sheaf and plot them together: {f}.\n\n\
                 1. Call plot with overlay_files [{list}] (patterns and folders work too). It imports each file and puts \
                 each file's Y columns into one graph; use template \"line\", or \"scatter\" for points.\n\
                 2. Call list_contents to see the workbooks, sheets, columns and the graph that were made.\n\
                 3. Look at the picture. If the axes need work, call plot again with graph set to the graph's name and \
                 style such as {{\"x_title\": \"E / V\", \"y_title\": \"I / uA\", \"x_min\": 0, \"x_max\": 1}}.\n\
                 4. Call export_graph with path \"overlay.png\" (or .svg, .pdf) and tell me where the file is.",
                f = values[0]
            )
        }
        "integrate_peak" => {
            let (c, a, b) = (q(&values[0]), &values[1], &values[2]);
            format!(
                "Integrate the peak of column {c0} between X = {a} and X = {b}.\n\n\
                 1. Call list_contents to find the sheet and the column; call read_columns for a few rows if you need \
                 to see the data.\n\
                 2. Call analyze with {{\"tool\": \"integrate\", \"columns\": [{c}], \"x_range\": \"{a}:{b}\", \
                 \"options\": {{\"end_point_baseline\": true}}}} (write a negative limit as \"-0.5:0.1\").\n\
                 3. Report the area, the peak position and height, and the half-height width from the table in the reply.\n\
                 4. If a figure helps, call plot with y [{c}] and style {{\"x_min\": {a}, \"x_max\": {b}}}, then \
                 export_graph, and look at the image.",
                c0 = values[0]
            )
        }
        _ => {
            let (c, m) = (q(&values[0]), &values[1]);
            format!(
                "Fit the model {m} to column {c0}.\n\n\
                 1. Call list_contents to find the sheet; call list_fit_models if you are unsure of the model's name \
                 or parameters.\n\
                 2. Call fit with {{\"model\": {mq}, \"y\": [{c}]}}. Restrict the points with \"x_range\" (for example \
                 \"0.4:0.9\") and give starting values with \"init\" (for example {{\"xc\": 0.45, \"w\": 0.05}}) if \
                 the fit does not converge.\n\
                 3. Report each parameter with its standard error and confidence limits, the R-square and the reduced \
                 chi-square, and say whether the fit is good.\n\
                 4. To see the curve over the data, call fit again with \"plot\" set to a path such as \"fit.png\", or \
                 call plot and export_graph.",
                c0 = values[0],
                mq = q(m)
            )
        }
    };
    Ok(json!({
        "description": def.description,
        "messages": [{"role": "user", "content": {"type": "text", "text": text}}]
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(v: Value) -> Map<String, Value> {
        v.as_object().unwrap().clone()
    }

    #[test]
    fn three_prompts_are_listed_with_their_arguments() {
        let l = list();
        let names: Vec<&str> = l["prompts"].as_array().unwrap().iter().map(|p| p["name"].as_str().unwrap()).collect();
        assert_eq!(names, ["plot_cv_files", "integrate_peak", "fit_curve"]);
        for p in l["prompts"].as_array().unwrap() {
            assert!(!p["arguments"].as_array().unwrap().is_empty());
            assert!(p["description"].as_str().unwrap().len() > 10);
        }
    }

    #[test]
    fn a_prompt_names_tools_and_options_that_exist() {
        let tools: Vec<String> = crate::schemas::defs().iter().map(|d| d.name.to_string()).collect();
        let cases = [
            ("plot_cv_files", json!({"files": "a.txt b.txt"})),
            ("integrate_peak", json!({"column": "Current", "from": 0.2, "to": "0.6"})),
            ("fit_curve", json!({"column": "B", "model": "Gauss"})),
        ];
        for (name, a) in cases {
            let r = get(name, &args(a)).unwrap();
            let text = r["messages"][0]["content"]["text"].as_str().unwrap();
            assert_eq!(r["messages"][0]["role"], "user");
            // Every word before an opening brace or after "Call" that looks like a tool is a real tool.
            for word in text.split(|c: char| !(c.is_ascii_alphanumeric() || c == '_')) {
                if word.contains('_') && word.chars().all(|c| c.is_ascii_lowercase() || c == '_') {
                    let option_like =
                        ["overlay_files", "x_title", "y_title", "x_min", "x_max", "x_range", "end_point_baseline"];
                    assert!(tools.iter().any(|t| t == word) || option_like.contains(&word), "{name}: {word}");
                }
            }
            assert!(text.contains("export_graph") || name != "plot_cv_files");
        }
        let r = get("integrate_peak", &args(json!({"column": "Current, mA", "from": "-0.5", "to": "0.1"}))).unwrap();
        let text = r["messages"][0]["content"]["text"].as_str().unwrap();
        assert!(text.contains("\"Current, mA\"") && text.contains("\"x_range\": \"-0.5:0.1\""), "{text}");
        assert!(text.contains("\"end_point_baseline\": true"));
        // The call the prompt shows is accepted by the tool's schema.
        let call = json!({"tool": "integrate", "columns": ["Current, mA"], "x_range": "-0.5:0.1", "options": {"end_point_baseline": true}});
        let def = crate::schemas::defs().into_iter().find(|d| d.name == "analyze").unwrap();
        for key in call.as_object().unwrap().keys() {
            assert!(def.schema["properties"].get(key).is_some(), "{key}");
        }
    }

    #[test]
    fn missing_arguments_and_unknown_prompts_are_refused() {
        assert!(get("fit_curve", &args(json!({"column": "B"}))).unwrap_err().contains("'model'"));
        assert!(get("plot_cv_files", &args(json!({"files": "  "}))).is_err());
        assert!(get("nope", &args(json!({}))).unwrap_err().contains("unknown prompt"));
    }
}
