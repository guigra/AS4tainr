**FREE
// Stage 2, S2.181: XML-SAX %HANDLER: the handler gets each event with
// the communication area, a pointer to the event's string and its
// length; a handler returning non-zero ends the parsing. Captured on
// PUB400 (tools/golden/native181.txt n181_sax) for the native unit
// NXMLSAX.
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
say(line);
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

dcl-proc say;
  dcl-pi *n;
    text varchar(200) const;
  end-pi;
  dcl-s wbuf varchar(200);
  wbuf = text;
  exec sql call qsys2.ifs_write_utf8(path_name => 'gn1.out', line => :wbuf,
                                     overwrite => 'APPEND', end_of_line => 'LF');
end-proc;
