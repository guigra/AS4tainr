**FREE
// Native unit NXMLSAX (S2.181): XML-SAX %HANDLER, the program captured on
// PUB400 as GN181SX with DSPLY for its output.
// Source: golden capture tools/golden/native181 n181_sax (PUB400,
// IBM i 7.5, 2026-11-21).
ctl-opt dftactgrp(*no);
dcl-ds info qualified inz;
  path varchar(100);
  attrs varchar(100);
  count int(10);
end-ds;
dcl-s doc varchar(200)
  inz('<order id="7"><item>pen</item><item>ink &amp; nib</item><stop/><item>x</item></order>');
dcl-s line varchar(200);

xml-sax(e) %handler(hdl : info) %xml(doc);
line = info.path + '|' + info.attrs + ' ' + %char(info.count)
     + ' ' + %char(%error) + ' ' + %char(%status);
dsply (line);
*inlr = *on;

dcl-proc hdl;
  dcl-pi *n int(10);
    c likeds(info);
    event int(10) value;
    pstr pointer value;
    len int(20) value;
    exc int(10) value;
  end-pi;
  dcl-s str char(100) based(pstr);
  dcl-s v varchar(100);
  if len > 0;
    v = %subst(str: 1: len);
  endif;
  c.count += 1;
  select;
  when event = *XML_START_ELEMENT;
    if v = 'stop';
      return 1;
    endif;
    c.path += '<' + v;
  when event = *XML_CHARS or event = *XML_PREDEF_REF;
    c.path += v;
  when event = *XML_ATTR_NAME;
    c.attrs += v + '=';
  when event = *XML_ATTR_CHARS;
    c.attrs += v;
  endsl;
  return 0;
end-proc;
