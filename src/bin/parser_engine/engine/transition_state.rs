use std::collections::HashMap;

use super::LookaheadTransition;

// lookahead transition (SymbolId -> LookaheadTransition)
static LA_TRANSITION_TABLE: std::sync::OnceLock<Vec<HashMap<u32, LookaheadTransition>>> = std::sync::OnceLock::new();
// Goto transition (RuleId -> state)
static GOTO_TRANSITION_TABLE: std::sync::OnceLock<Vec<Option<HashMap<u32, usize>>>> = std::sync::OnceLock::new();

pub fn init_lookahead_translations() -> &'static Vec<HashMap<u32, LookaheadTransition>> {
    LA_TRANSITION_TABLE.get_or_init(|| {
        let mut la_entries = vec![];
        la_entries.resize(748, Default::default());
        
        // ** Shift/state: 0 -> 18, eat: SELECT (None)
        la_entries[0] = maplit::hashmap!(
            173u32 => LookaheadTransition::Reduce{ pop_count: 0, lhs: 236 },
            175u32 => LookaheadTransition::Reduce{ pop_count: 0, lhs: 236 },
            176u32 => LookaheadTransition::Reduce{ pop_count: 0, lhs: 236 },
            190u32 => LookaheadTransition::Shift{ next_state: 17 },
            201u32 => LookaheadTransition::Shift{ next_state: 18 },
        );
        // ** Reduce/state: 18, name: distinct, rhs/len: 0, goto: 18 -> 72
        la_entries[18] = maplit::hashmap!(
            85u32 =>  LookaheadTransition::Reduce{ pop_count: 0, lhs: 202 },
            198u32 => LookaheadTransition::Shift{ next_state: 70 },
            215u32 => LookaheadTransition::Shift{ next_state: 71 },
        );
        // ** Reduce/state: 72, name: sclp, rhs/len: 0, goto: 145
        la_entries[72] = maplit::hashmap!(
            85u32 =>  LookaheadTransition::Reduce{ pop_count: 0, lhs: 216 }, 
        );
        // ** Reduce/state: 145, name: scanpt, rhs/len: 0, goto: 239
        la_entries[145] = maplit::hashmap!(
            85u32 =>  LookaheadTransition::Reduce{ pop_count: 0, lhs: 152 }, 
        );
        // ** Shift/state: 239 -> 111, eat: ID (Some("c"))
        la_entries[239] = maplit::hashmap!(
            61u32 =>  LookaheadTransition::Shift{ next_state: 110 },
            85u32 =>  LookaheadTransition::Shift{ next_state: 111 },
            159u32 => LookaheadTransition::Shift{ next_state: 120 },
        );
        // ** Reduce/state: 111, name: nm, rhs/len: 1, goto: 363
        la_entries[111] = maplit::hashmap!(
            145u32 => LookaheadTransition::Reduce{ pop_count: 1, lhs: 158 },
            218u32 => LookaheadTransition::Reduce{ pop_count: 1, lhs: 16 },
            221u32 => LookaheadTransition::Reduce{ pop_count: 1, lhs: 158 },
        );
        // ** Shift/state: 363 -> 469, eat: DOT (Some("."))
        la_entries[363] = maplit::hashmap!(
            218u32 => LookaheadTransition::Shift{ next_state: 469 },
        );
        // ** Shift/state: 469 -> 45, eat: ID (Some("code"))
        la_entries[469] = maplit::hashmap!(
            85u32  => LookaheadTransition::Shift{ next_state: 45 },
            134u32 => LookaheadTransition::Shift{ next_state: 581 },
            145u32 => LookaheadTransition::Shift{ next_state: 47 },      
        );
        // ** Reduce/state: 45, name: nm, rhs/len: 1, goto: 340
        la_entries[45] = maplit::hashmap!(
            47u32  => LookaheadTransition::Reduce{ pop_count: 1, lhs: 16 },
            85u32  => LookaheadTransition::Reduce{ pop_count: 1, lhs: 16 },
            134u32 => LookaheadTransition::Reduce{ pop_count: 1, lhs: 16 },
            44u32 =>  LookaheadTransition::Reduce{ pop_count: 1, lhs: 16 },
            227u32 => LookaheadTransition::Reduce{ pop_count: 1, lhs: 16 },
        );
        // ** Reduce/state: 340, name: expr, rhs/len: 3, goto: 362
        la_entries[340] = maplit::hashmap!(
            44u32 =>  LookaheadTransition::Reduce{ pop_count: 3, lhs: 158 },
            47u32 =>  LookaheadTransition::Reduce{ pop_count: 3, lhs: 158 },
            221u32 => LookaheadTransition::Reduce{ pop_count: 3, lhs: 158 },
        );
        // ** Reduce/state: 362, name: scanpt, rhs/len: 0, goto: 468
        la_entries[362] = maplit::hashmap!(
            44u32 =>  LookaheadTransition::Reduce{ pop_count: 0, lhs: 152 },
            47u32 =>  LookaheadTransition::Reduce{ pop_count: 0, lhs: 152 },
            221u32 => LookaheadTransition::Reduce{ pop_count: 0, lhs: 152 },
            68u32 =>  LookaheadTransition::Shift{ next_state: 194 },
            69u32 =>  LookaheadTransition::Shift{ next_state: 195 },
        );
        // ** Reduce/state: 468, name: as, rhs/len: 0, goto: 580
        la_entries[468] = maplit::hashmap!(
            44u32 =>  LookaheadTransition::Shift{ next_state: 577 },
            47u32 =>  LookaheadTransition::Reduce{ pop_count: 0, lhs: 217 },       
            221u32 => LookaheadTransition::Reduce{ pop_count: 0, lhs: 217 },       
        );
        // ** Reduce/state: 580, name: selcollist, rhs/len: 5, goto: 146
        la_entries[580] = maplit::hashmap!(
            42u32 =>  LookaheadTransition::Reduce{ pop_count: 5, lhs: 203 },
            47u32 =>  LookaheadTransition::Reduce{ pop_count: 5, lhs: 203 },        
            221u32 => LookaheadTransition::Reduce{ pop_count: 5, lhs: 203 },        
        );
        // ** Shift/state: 146 -> 240, eat: COMMA (Some(","))
        la_entries[146] = maplit::hashmap!(
            42u32 =>  LookaheadTransition::Reduce{ pop_count: 0, lhs: 204 },
            47u32 =>  LookaheadTransition::Shift{ next_state: 240 },
            221u32 => LookaheadTransition::Shift{ next_state: 241 },
        );
        // ** Reduce/state: 240, name: sclp, rhs/len: 2, goto: 145
        la_entries[240] = maplit::hashmap!(
            85u32 =>  LookaheadTransition::Reduce{ pop_count: 2, lhs: 216 },
            132u32 => LookaheadTransition::Reduce{ pop_count: 2, lhs: 216 },
            159u32 => LookaheadTransition::Reduce{ pop_count: 2, lhs: 216 },
        );
        // ** Reduce/state: 145, name: scanpt, rhs/len: 0, goto: 239
        // la_entries[145] = maplit::hashmap!(
        // );
        // ** Shift/state: 239 -> 111, eat: ID (Some("name"))
        // la_entries[239] = maplit::hashmap!(
        // );
        // ** Reduce/state: 111, name: expr, rhs/len: 1, goto: 362
        // la_entries[111] = maplit::hashmap!(
        // );
        // ** Reduce/state: 362, name: scanpt, rhs/len: 0, goto: 468
        // la_entries[362] = maplit::hashmap!(
        // );
        // ** Reduce/state: 468, name: as, rhs/len: 0, goto: 580
        // la_entries[468] = maplit::hashmap!(
        // );
        // ** Reduce/state: 580, name: selcollist, rhs/len: 5, goto: 146
        // la_entries[580] = maplit::hashmap!(
        // );
        // ** Shift/state: 146 -> 241, eat: FROM (None)
        // la_entries[146] = maplit::hashmap!(
        // );
        // ** Reduce/state: 241, name: stl_prefix, rhs/len: 0, goto: 365
        la_entries[241] = maplit::hashmap!(
            85u32 =>  LookaheadTransition::Reduce{ pop_count: 0, lhs: 220 },
        );
        // ** Shift/state: 365 -> 45, eat: ID (Some("city"))
        la_entries[365] = maplit::hashmap!(
            39u32 =>  LookaheadTransition::Shift{ next_state: 474 },
            85u32 =>  LookaheadTransition::Shift{ next_state: 45 },        
        );
        // ** Reduce/state: 45, name: nm, rhs/len: 1, goto: 475
        // ** Reduce/state: 475, name: dbnm, rhs/len: 0, goto: 586
        la_entries[475] = maplit::hashmap!(
            85u32 =>  LookaheadTransition::Reduce{ pop_count: 0, lhs: 33 },
            238u32 => LookaheadTransition::Reduce{ pop_count: 0, lhs: 33 },
        );
        // ** Shift/state: 586 -> 578, eat: ID (Some("c"))
        la_entries[586] = maplit::hashmap!(
            47u32  => LookaheadTransition::Reduce{ pop_count: 0, lhs: 217 },
            85u32  => LookaheadTransition::Shift{ next_state: 578 },        
            238u32 => LookaheadTransition::Reduce{ pop_count: 0, lhs: 217 },
        );
        // ** Reduce/state: 578, name: as, rhs/len: 1, goto: 668
        la_entries[578] = maplit::hashmap!(
            5u32 =>   LookaheadTransition::Reduce{ pop_count: 1, lhs: 217 },
            47u32 =>  LookaheadTransition::Reduce{ pop_count: 1, lhs: 217 },   
            238u32 => LookaheadTransition::Reduce{ pop_count: 1, lhs: 217 },
        );
        // ** Reduce/state: 668, name: on_using, rhs/len: 0, goto: 741
        la_entries[668] = maplit::hashmap!(
            5u32 =>   LookaheadTransition::Reduce{ pop_count: 0, lhs: 223 },
            143u32 => LookaheadTransition::Shift{ next_state: 422 },         
            238u32 => LookaheadTransition::Reduce{ pop_count: 0, lhs: 223 },    
        );
        // ** Reduce/state: 741, name: seltablist, rhs/len: 5, goto: 364
        la_entries[741] = maplit::hashmap!(
            5u32 =>   LookaheadTransition::Reduce{ pop_count: 5, lhs: 219 },
            238u32 => LookaheadTransition::Reduce{ pop_count: 5, lhs: 219 },     
        );
        // ** Reduce/state: 364, name: from, rhs/len: 2, goto: 242
        la_entries[364] = maplit::hashmap!(
            5u32 =>   LookaheadTransition::Reduce{ pop_count: 2, lhs: 204 },
            227u32 => LookaheadTransition::Shift{ next_state: 472 },
            231u32 => LookaheadTransition::Reduce{ pop_count: 2, lhs: 204 },
        );
        // ** Reduce/state: 242, name: where_opt, rhs/len: 0, goto: 367
        la_entries[242] = maplit::hashmap!(
            5u32 =>   LookaheadTransition::Reduce{ pop_count: 0, lhs: 205 },  
            231u32 => LookaheadTransition::Reduce{ pop_count: 0, lhs: 205 },         
            238u32 => LookaheadTransition::Shift{  next_state: 366 },          
        );
        // ** Reduce/state: 367, name: groupby_opt, rhs/len: 0, goto: 478
        la_entries[367] = maplit::hashmap!(
            5u32 =>   LookaheadTransition::Reduce{ pop_count: 0, lhs: 206 }, 
            197u32 => LookaheadTransition::Reduce{ pop_count: 0, lhs: 206 },       
            233u32 => LookaheadTransition::Shift{  next_state: 477 },
        );
        // ** Reduce/state: 478, name: having_opt, rhs/len: 0, goto: 589
        la_entries[478] = maplit::hashmap!(
            5u32 =>   LookaheadTransition::Reduce{ pop_count: 0, lhs: 207 }, 
            199u32 => LookaheadTransition::Reduce{ pop_count: 0, lhs: 207 },       
            234u32 => LookaheadTransition::Shift{  next_state: 588 },
        );
        // ** Reduce/state: 589, name: orderby_opt, rhs/len: 0, goto: 673
        la_entries[589] = maplit::hashmap!(
            5u32 =>   LookaheadTransition::Reduce{ pop_count: 0, lhs: 208 }, 
            197u32 => LookaheadTransition::Reduce{ pop_count: 0, lhs: 208 }, 
            231u32 => LookaheadTransition::Shift{  next_state: 671 },          
        );
        // ** Reduce/state: 673, name: limit_opt, rhs/len: 0, goto: 747
        la_entries[673] = maplit::hashmap!(
            5u32 =>   LookaheadTransition::Reduce{ pop_count: 0, lhs: 209 }, 
            200u32 => LookaheadTransition::Reduce{ pop_count: 0, lhs: 209 }, 
            235u32 => LookaheadTransition::Shift{  next_state: 746 },       
        );
        // ** Reduce/state: 747, name: oneselect, rhs/len: 9, goto: 31
        la_entries[747] = maplit::hashmap!(
            5u32 =>   LookaheadTransition::Reduce{ pop_count: 9, lhs: 194 },
            200u32 => LookaheadTransition::Reduce{ pop_count: 9, lhs: 194 },
        );
        // ** Reduce/state: 31, name: selectnowith, rhs/len: 1, goto: 33
        la_entries[31] = maplit::hashmap!(
            5u32 =>   LookaheadTransition::Reduce{ pop_count: 1, lhs: 193 },
            197u32 => LookaheadTransition::Reduce{ pop_count: 1, lhs: 193 },
        );
        // ** Reduce/state: 33, name: select, rhs/len: 1, goto: 32
        la_entries[33] = maplit::hashmap!(
            5u32 =>   LookaheadTransition::Reduce{ pop_count: 1, lhs: 45 },
            197u32 => LookaheadTransition::Shift{  next_state: 90 },
            200u32 => LookaheadTransition::Shift{  next_state: 91 },          
            200u32 => LookaheadTransition::Shift{  next_state: 92 },
        );
        // ** Reduce/state: 32, name: cmd, rhs/len: 1, goto: 21
        la_entries[32] = maplit::hashmap!(
            5u32 => LookaheadTransition::Reduce{ pop_count: 1, lhs: 11 },
        );
        // ** Reduce/state: 21, name: cmdx, rhs/len: 1, goto: 23
        la_entries[21] = maplit::hashmap!(
            5u32 => LookaheadTransition::Reduce{ pop_count: 1, lhs: 6 },
        );
        // ** Shift/state: 23 -> 76, eat: SEMI (Some(";"))
        la_entries[23] = maplit::hashmap!(
            5u32 => LookaheadTransition::Shift{  next_state: 76 },
        );
        // ** Reduce/state: 76, name: ecmd, rhs/len: 2, goto: 27
        la_entries[76] = maplit::hashmap!(
            173u32 => LookaheadTransition::Reduce{ pop_count: 2, lhs: 4 },
            191u32 => LookaheadTransition::Reduce{ pop_count: 2, lhs: 4 },     
        );
        // ** Reduce/state: 27, name: cmdlist, rhs/len: 1, goto: 22
        la_entries[27] = maplit::hashmap!(
            173u32 => LookaheadTransition::Reduce{ pop_count: 1, lhs: 3 },
            175u32 => LookaheadTransition::Reduce{ pop_count: 1, lhs: 3 },     
            191u32 => LookaheadTransition::Reduce{ pop_count: 1, lhs: 3 },   
        );
        // ** Reduce/state: 22, name: input, rhs/len: 1, goto: 29
        la_entries[22] = maplit::hashmap!(
            190u32 => LookaheadTransition::Shift{  next_state: 17 },
            191u32 => LookaheadTransition::Reduce{ pop_count: 1, lhs: 2 },      
            201u32 => LookaheadTransition::Shift{  next_state: 18 },      
        );
        // ** Shift/state: 29 -> 88, eat: EOF (None)
        la_entries[29] = maplit::hashmap!(
            191u32 => LookaheadTransition::Shift{  next_state: 88 },
        );

        la_entries
    })
}

// ** Accept/state: 88
pub fn eof_transition() -> (usize, usize) {
    (88, 329)
}

pub fn init_goto_transition_table() -> &'static Vec<Option<HashMap<u32, usize>>> {
    GOTO_TRANSITION_TABLE.get_or_init(|| {
        let mut goto_entries = Vec::with_capacity(670);
        goto_entries.resize(748, None);

        // ** Shift/state: 0 -> 18, eat: SELECT (None)
        goto_entries[0] = Some(maplit::hashmap!(
            7u32 => 28,
            236u32 => 35,
        ));
        // ** Reduce/state: 18, name: distinct, rhs/len: 0, goto: 18 -> 72
        goto_entries[18] = Some(maplit::hashmap!(
            202u32 => 72,
        ));
        // ** Reduce/state: 72, name: sclp, rhs/len: 0, goto: 72 -> 145
        goto_entries[72] = Some(maplit::hashmap!(
            216u32 => 145,
            203u32 => 146,
        ));
        // ** Reduce/state: 145, name: scanpt, rhs/len: 0, goto: 145 -> 239
        goto_entries[145] = Some(maplit::hashmap!(
            152u32 => 239,
        ));
        // ** Shift/state: 239 -> 111, eat: ID (Some("c"))

        // ** Reduce/state: 111, name: nm, rhs/len: 1, goto: 239 -> 363
        // ** Shift/state: 363 -> 469, eat: DOT (Some("."))
        // ** Shift/state: 469 -> 45, eat: ID (Some("code"))
        // ** Reduce/state: 45, name: nm, rhs/len: 1, goto: 469 -> 340
        goto_entries[469] = Some(maplit::hashmap!(
            16 => 340,
        ));
        // ** Reduce/state: 340, name: expr, rhs/len: 3, goto: 239 -> 362
        goto_entries[239] = Some(maplit::hashmap!(
            158 => 362,
            16 => 363,
            157 => 129,
        ));
        // ** Reduce/state: 362, name: scanpt, rhs/len: 0, goto: 362 -> 468
        goto_entries[362] = Some(maplit::hashmap!(
            254u32 => 223,
            252u32 => 224,
            152u32 => 468,
        ));
        // ** Reduce/state: 468, name: as, rhs/len: 0, goto: 468 -> 580
        goto_entries[468] = Some(maplit::hashmap!(
            217u32 => 580,
        ));
        // ** Reduce/state: 580, name: selcollist, rhs/len: 5, goto: 72 -> 146
        // ** Shift/state: 146 -> 240, eat: COMMA (Some(","))
        // ** Reduce/state: 240, name: sclp, rhs/len: 2, goto: 72 -> 145

        // ** Reduce/state: 145, name: scanpt, rhs/len: 0, goto: 145 -> 239
        // goto_entries[145] = Some(maplit::hashmap!(
        // ));
        // ** Shift/state: 239 -> 111, eat: ID (Some("name"))
        // ** Reduce/state: 111, name: expr, rhs/len: 1, goto: 239 -> 362
        // ** Reduce/state: 362, name: scanpt, rhs/len: 0, goto: 362 -> 468
        // goto_entries[362] = Some(maplit::hashmap!(
        // ));
        // ** Reduce/state: 468, name: as, rhs/len: 0, goto: 468 -> 580
        // goto_entries[468] = Some(maplit::hashmap!(
        // ));
        // ** Reduce/state: 580, name: selcollist, rhs/len: 5, goto: 72 -> 146
        // goto_entries[580] = Some(maplit::hashmap!(
        // ));
        // ** Shift/state: 146 -> 241, eat: FROM (None)
        
        // ** Reduce/state: 241, name: stl_prefix, rhs/len: 0, goto: 241 -> 365
        goto_entries[241] = Some(maplit::hashmap!(
            219u32 => 364,
            220u32 => 365,
        ));
        // ** Shift/state: 365 -> 45, eat: ID (Some("city"))
        
        // ** Reduce/state: 45, name: nm, rhs/len: 1, goto: 365 -> 475
        goto_entries[365] = Some(maplit::hashmap!(
            16 => 475,
        ));
        // ** Reduce/state: 475, name: dbnm, rhs/len: 0, goto: 475 -> 586
        goto_entries[475] = Some(maplit::hashmap!(
            33 => 586,
        ));
        // ** Shift/state: 586 -> 578, eat: ID (Some("c"))
        // ** Reduce/state: 578, name: as, rhs/len: 1, goto: 586 -> 668
        goto_entries[586] = Some(maplit::hashmap!(
            217 => 668,
        ));
        // ** Reduce/state: 668, name: on_using, rhs/len: 0, goto: 668 -> 741
        goto_entries[668] = Some(maplit::hashmap!(
            224u32 => 740,
            223u32 => 741,
        ));
        // ** Reduce/state: 741, name: seltablist, rhs/len: 5, goto: 241 -> 364
        // ** Reduce/state: 364, name: from, rhs/len: 2, goto: 146 -> 242
        goto_entries[146] = Some(maplit::hashmap!(
            204 => 242,
        ));
        // ** Reduce/state: 242, name: where_opt, rhs/len: 0, goto: 242 -> 367
        goto_entries[242] = Some(maplit::hashmap!(
            205u32 => 367,
        ));
        // ** Reduce/state: 367, name: groupby_opt, rhs/len: 0, goto: 367 -> 478
        goto_entries[367] = Some(maplit::hashmap!(
            206u32 => 478,
        ));
        // ** Reduce/state: 478, name: having_opt, rhs/len: 0, goto: 478 -> 589
        goto_entries[478] = Some(maplit::hashmap!(
            207u32 => 589,
        ));
        // ** Reduce/state: 589, name: orderby_opt, rhs/len: 0, goto: 589 -> 673
        goto_entries[589] = Some(maplit::hashmap!(
            208u32 => 673,
            210u32 => 674,
        ));
        // ** Reduce/state: 673, name: limit_opt, rhs/len: 0, goto: 673 -> 747
        goto_entries[673] = Some(maplit::hashmap!(
            209u32 => 747,
        ));
        // ** Reduce/state: 747, name: oneselect, rhs/len: 9, goto: 0 -> 31
        goto_entries[0] = Some(maplit::hashmap!(
            6 => 23,
            7 => 28,
            4 => 27,
            45 => 32,
            194 => 31,
            236 => 35,
            11 => 21,
            2 => 29,
            3 => 22,
            193 => 33,
        ));

        // ** Reduce/state: 31, name: selectnowith, rhs/len: 1, goto: 0 -> 33
        // ** Reduce/state: 33, name: select, rhs/len: 1, goto: 0 -> 32
        // ** Reduce/state: 32, name: cmd, rhs/len: 1, goto: 0 -> 21
        // ** Reduce/state: 21, name: cmdx, rhs/len: 1, goto: 0 -> 23
        // ** Shift/state: 23 -> 76, eat: SEMI (Some(";"))
        // ** Reduce/state: 76, name: ecmd, rhs/len: 2, goto: 0 -> 27
        // ** Reduce/state: 27, name: cmdlist, rhs/len: 1, goto: 0 -> 22
        // ** Reduce/state: 22, name: input, rhs/len: 1, goto: 0 -> 29
        // ** Shift/state: 29 -> 88, eat: EOF (None)
        
        goto_entries
    })
}
