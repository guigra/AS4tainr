      * Stage 2, S2.191: READ of a program-described DISK file with input
      * specifications: the record identifying indicators by the code in
      * position 1, the input fields (character and zoned with decimals)
      * from the record image. Captured on PUB400 (tools/golden/native191.txt
      * n191_pdread) for the native unit NPDREAD.
     H DFTACTGRP(*NO)
     FGN191F    IF   F   20        DISK    USROPN EXTFILE('QTEMP/GN191F')
     D line            S            200    VARYING
     D say             PR
     D  text                        200    VARYING CONST
     IGN191F    NS  01    1 CH
     I                                  2   11  NAME
     IGN191F    NS  02    1 CD
     I                                  2    4 0QTY
     I                                  5    9 2PRICE
      /free
        exec sql set option commit = *none;
        exec sql drop table QTEMP/GN191F;
        exec sql create table QTEMP/GN191F (r char(20) not null);
        exec sql insert into QTEMP/GN191F
                 values ('Habc'), ('D0120350'), ('Hxyz'), ('D9990001');
        open gn191f;
        read gn191f;
        dow not %eof(gn191f);
          if *in01;
            line += 'H' + %trim(name) + ' ';
          endif;
          if *in02;
            line += 'D' + %char(qty) + '/' + %char(price) + ' ';
          endif;
          *in01 = *off;
          *in02 = *off;
          read gn191f;
        enddo;
        close gn191f;
        line += %char(%eof(gn191f));
        say(line);
        *inlr = *on;
      /end-free
     P say             B
     D say             PI
     D  text                        200    VARYING CONST
     D wbuf            S            200    VARYING
      /free
        wbuf = text;
        exec sql call qsys2.ifs_write_utf8(path_name => 'gn1.out',
                 line => :wbuf, overwrite => 'APPEND', end_of_line => 'LF');
      /end-free
     P say             E
