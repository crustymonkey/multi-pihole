var MPI = {
  enable: function () {
    $.ajax({
      url: 'https://pi-ctl.splitstreams.com/enable',
      error: function(xhr, stat, err) {
        alert('An error occured (' + stat + ') enabling piholes: ' + err);
      },
      success: function(data, stat, xhr) {
        alert('Successfully enabled the pihole servers');
      }
    });
  },
  disable: function (seconds) {
    $.ajax({
      url: 'https://pi-ctl.splitstreams.com/disable/' + seconds,
      error: function(xhr, stat, err) {
        alert('An error occured (' + stat + ') disabling piholes: ' + err);
      },
      success: function(data, stat, xhr) {
        alert('Successfully disabled the pihole servers for ' + seconds +
          ' seconds');
      }
    });
  }
};
