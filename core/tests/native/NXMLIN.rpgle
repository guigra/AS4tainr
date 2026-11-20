**FREE
// Native unit NXMLIN (S2.165): XML-INTO, the program captured on PUB400
// as GN165XI with DSPLY for its output.
// Source: golden capture tools/golden/native165 n165_xmlinto (PUB400,
// IBM i 7.5, 2026-11-20).
ctl-opt dftactgrp(*no);
dcl-ds cust qualified;
  id int(10);
  name varchar(20);
end-ds;
dcl-ds item qualified dim(3);
  code char(3);
  qty packed(5:0);
end-ds;
dcl-s doc varchar(200);
dcl-s line varchar(120);

doc = '<cust><id>7</id><name>Ann</name></cust>';
xml-into cust %xml(doc);
line = %char(cust.id) + ' ' + cust.name;
doc = '<order><List><Item><code>A1</code><qty>3</qty></Item>'
    + '<Item><code>B2</code><qty>12</qty></Item></List></order>';
xml-into item %xml(doc : 'path=order/list/item case=any');
line += ' ' + %trim(item(1).code) + '=' + %char(item(1).qty)
      + ' ' + %trim(item(2).code) + '=' + %char(item(2).qty)
      + ' [' + item(3).code + ']';
dsply (line);
*inlr = *on;
