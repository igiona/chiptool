use log::*;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

use super::common::*;
use crate::ir::*;

#[derive(Debug, Serialize, Deserialize)]
pub struct MergeEnums {
    pub from: String,
    pub to: String,
    #[serde(default)]
    pub check: CheckLevel,
    #[serde(default)]
    pub skip_unmergeable: bool,
    pub keep_desc: Option<bool>,
}

impl MergeEnums {
    pub fn run(&self, ir: &mut IR) -> anyhow::Result<()> {
        if self.keep_desc.unwrap_or(false) {
            let variant_desc = extract_variant_desc(ir, &self.from, None)?;
            append_variant_desc_to_field(ir, &variant_desc, None);
        }

        let re = make_regex(&self.from)?;
        let groups = match_groups(ir.enums.keys().cloned(), &re, &self.to);

        for (to, group) in groups {
            info!("Merging enums, dest: {}", to);
            for id in &group {
                info!("   {}", id);
            }
            self.merge_enums(ir, group, to)?;
        }

        Ok(())
    }

    fn merge_enums(&self, ir: &mut IR, ids: BTreeSet<String>, to: String) -> anyhow::Result<()> {
        let e = ir.enums.get(ids.iter().next().unwrap()).unwrap().clone();

        for id in &ids {
            let e2 = ir.enums.get(id).unwrap();
            if let Err(e) = check_mergeable_enums(&e, e2, self.check) {
                if self.skip_unmergeable {
                    info!("skipping: {:?}", to);
                    return Ok(());
                } else {
                    return Err(e);
                }
            }
        }
        for id in &ids {
            ir.enums.remove(id);
        }

        assert!(ir.enums.insert(to.clone(), e).is_none());
        replace_enum_ids(ir, &ids, to);

        Ok(())
    }
}
