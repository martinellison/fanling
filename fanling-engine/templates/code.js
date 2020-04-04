/* This Source Code Form is subject to the terms of the Mozilla Public
License, v. 2.0. If a copy of the MPL was not distributed with this
file, You can obtain one at https://mozilla.org/MPL/2.0/. */
console.log("starting scripts");
window.addEventListener("load", function() {doAction("Start", "", "");});
var setTag = function(tag, text) {
    var s = document.getElementById(tag);
    if (!s) console.error("no tag called '" + tag + "'");
    else s.innerHTML = text;
};
var doAction = function(aVal, tVal, iVal) {
    var c = {
       a:aVal, i:iVal, t: tVal
    };
    invoke(c);
};
var doActionWithIdent = function(aVal, tVal, iVal, iVal2) {
    var a = {};
    a[aVal] = iVal2;
    doAction(a, tVal, iVal);
};
{% match interface_type %}
{% when crate::InterfaceType::PC %}
// code for PC platform
var invoke = function(arg) {
  console.log("\ninvoking from PC with "+JSON.stringify(arg));
  window.external.invoke(JSON.stringify(arg));
};
{% when crate::InterfaceType::Android %}
// android specific code
var invoke = function(arg) {
    console.log("\ninvoking from Android with " + JSON.stringify(arg));
    taipo.execute(JSON.stringify(arg));
    console.log("execute done");
    if (taipo.response_ok()) {
        console.log("response ok from execute");
        var nri =  taipo.response_num_items();
        for (i = 0 ; i < nri; i++) {
            let ri = taipo.response_item(i);
            let key = taipo.response_key(ri);
            let value = taipo.response_value(ri);
            console.log(i + ": "+key + " = "+value);
            setTag(key, value);
        }
    } else {
        console.log("system error found: " + taipo.response_error());
        alert( taipo.response_error());
        console.log("error: " + taipo.response_error());
    }
};
{% endmatch %}
var onclick_simple = function(ident, nextop) {
    let type_name = "Simple";
    // let base = {ident: ident, type: type_name };
    // if  (document.getElementById("parent").value != "")
    //     base.parent = document.getElementById("parent").value;
    // if  (document.getElementById("sort").value != "")
    //     base.sort = document.getElementById("sort").value;
    // if (document.getElementById("canbeparent").checked)
    //     base.can_be_parent = true;
    let base =  make_base(ident,  type_name);
    let data = {
        name: document.getElementById("name").value,
        text: document.getElementById("text").value,
    };
    // let action = {};
    // action[nextop] = [base, data];
    // var cmnd = { t: type_name, i: ident, a: action}; // TODO: tasks
    // invoke(cmnd);
    invoke_action(nextop, base, data, ident,  type_name );
};
var onclick_task = function(ident, nextop) {
    let type_name = "Task";
    let base =  make_base(ident,  type_name);
    let data = {
        name: document.getElementById("name").value,
        text: document.getElementById("text").value,
        priority: document.getElementById("priority").value,
        context: document.getElementById("context").value,
        deadline: document.getElementById("deadline").value,
        show_after_date: document.getElementById("showafterdate").value
    };
    invoke_action(nextop, base, data, ident,  type_name );
};
var make_base = function(ident,  type_name) {
    let base = {ident: ident, type: type_name };
    if  (document.getElementById("parent").value != "")
        base.parent = document.getElementById("parent").value;
    if  (document.getElementById("sort").value != "")
        base.sort = document.getElementById("sort").value;
    if (document.getElementById("canbeparent").checked)
        base.can_be_parent = true;
    if (document.getElementById("canbecontext").checked)
        base.can_be_context = true;
    return base;
}
var invoke_action = function(nextop, base, data, ident,  type_name ) {
    let action = {};
    action[nextop] = [base, data];
    var cmnd = { t: type_name, i: ident, a: action};
    invoke(cmnd);
}
var onclick_caret = function(elt) {
    console.log("click on caret " + elt);
    if (!elt) console.error("no element"); else console.log("elt is " + elt);
    if (!elt.parentElement) console.error("no parent"); else console.log("parent is " + elt.parentElement);
    var nested = elt.parentElement.querySelector(".nested");
    console.log("nested is "+nested);
    if (!nested) console.error("no nested"); else
   nested.classList.toggle("active");
    elt.classList.toggle("caret-down");
};

