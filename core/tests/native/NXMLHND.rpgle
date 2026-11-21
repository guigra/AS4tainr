**FREE
// Native unit NXMLHND (S2.180): XML-INTO %HANDLER, the program captured on
// PUB400 as GN180XH with DSPLY for its output.
// Source: golden capture tools/golden/native180 n180_handler (PUB400,
// IBM i 7.5, 2026-11-20).
ctl-opt dftactgrp(*no);
dcl-ds itemT qualified template;
  name char(10);
  qty packed(5:0);
end-ds;
dcl-s total packed(7:0);
dcl-s calls int(10);
dcl-s names varchar(30);
dcl-s stopAt int(10);
dcl-s xml varchar(500);
dcl-s line varchar(80);

xml = '<order><item><name>a</name><qty>2</qty></item><item><name>b</name>'
    + '<qty>3</qty></item><item><name>c</name><qty>4</qty></item></order>';
xml-into %handler(take : total) %xml(xml : 'path=order/item');
line = %char(total) + ' ' + %char(calls) + ' ' + names;
total = 0;
calls = 0;
names = '';
stopAt = 1;
xml-into(e) %handler(take : total) %xml(xml : 'path=order/item');
line += '|' + %char(total) + ' ' + %char(calls) + ' ' + names
      + ' ' + %char(%error) + ' ' + %char(%status);
dsply (line);
*inlr = *on;

dcl-proc take;
  dcl-pi *n int(10);
    tot packed(7:0);
    items likeds(itemT) dim(2) const;
    n uns(10) value;
  end-pi;
  dcl-s i int(10);
  calls += 1;
  for i = 1 to n;
    tot += items(i).qty;
    names += %trim(items(i).name);
  endfor;
  if stopAt > 0 and calls >= stopAt;
    return 1;
  endif;
  return 0;
end-proc;
