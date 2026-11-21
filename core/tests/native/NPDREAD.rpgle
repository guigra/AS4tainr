      * Native unit NPDREAD (S2.191): READ of a program-described file with
      * input specifications, the program captured on PUB400 as GN191PD with
      * DSPLY for its output.
      * Source: golden capture tools/golden/native191 n191_pdread (PUB400,
      * IBM i 7.5, 2026-11-21).
     H DFTACTGRP(*NO)
     FGN191F    IF   F   20        DISK    USROPN EXTFILE('QTEMP/GN191F')
     D line            S            200    VARYING
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
        dsply line;
        *inlr = *on;
      /end-free
