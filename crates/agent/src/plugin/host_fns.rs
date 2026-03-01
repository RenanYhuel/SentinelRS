use wasmtime::{Caller, Linker};
use super::host_state::HostState;
use super::error::PluginError;

fn extract_string(caller: &mut Caller<'_, HostState>, ptr: i32, len: i32) -> Option<String> {
    let memory = caller.get_export("memory")?.into_memory()?;
    let data = memory.data(&caller);
    let start = ptr as usize;
    let end = start.checked_add(len as usize)?;
    if end > data.len() {
        return None;
    }
    std::str::from_utf8(&data[start..end]).ok().map(String::from)
}

pub fn register_host_fns(linker: &mut Linker<HostState>) -> Result<(), PluginError> {
    linker
        .func_wrap("sentinel", "log", |mut caller: Caller<'_, HostState>, ptr: i32, len: i32| {
            if let Some(msg) = extract_string(&mut caller, ptr, len) {
                caller.data_mut().logs.push(msg);
            }
        })
        .map_err(|e| PluginError::Instantiation(e.to_string()))?;

    linker
        .func_wrap(
            "sentinel",
            "emit_metric_json",
            |mut caller: Caller<'_, HostState>, ptr: i32, len: i32| -> i32 {
                let max = caller.data().max_metrics;
                if caller.data().collected_json.len() as u64 >= max {
                    return -1;
                }
                match extract_string(&mut caller, ptr, len) {
                    Some(json) => {
                        caller.data_mut().collected_json.push(json);
                        0
                    }
                    None => -1,
                }
            },
        )
        .map_err(|e| PluginError::Instantiation(e.to_string()))?;

    Ok(())
}
