**FREE
// Stage 2, S2.180: XML-INTO %HANDLER: the path's elements into the
// handler's array parameter two at a time, the handler given the
// communication area, the array and how many were filled; a second run
// whose handler returns non-zero at its first call, which ends the
// operation. Captured on PUB400 (tools/golden/native180.txt n180_handler)
// for the native unit NXMLHND.
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
say(line);
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

dcl-proc say;
  dcl-pi *n;
    text varchar(200) const;
  end-pi;
  dcl-s wbuf varchar(200);
  wbuf = text;
  exec sql call qsys2.ifs_write_utf8(path_name => 'gn1.out', line => :wbuf,
                                     overwrite => 'APPEND', end_of_line => 'LF');
end-proc;
