# Shoes.app(title: "Welcome", width: 500, height: 600) do ...
game do
    # modifies game object to have this field
    define shared_score: 0

    # When you load a game.
    on_load do
      say :Start
    end
    # on_quit ?
    # on_pause/menu ?
    # on_new_game

    #shorthand for player.on_collect
    player do
      define num_gems: 0, num_tickets: 1
      #on_create
      #on_spawn
      on_collect :gems do
        @num_gems += 1
        map.show :big_gems if @num_gems > 3
      end
      on_collect :big_gems { @num_gems += 5 }
      on_collect :any_gem {
        total = players.map {|p| p.num_gems }.sum # or game.shared_score
        map.gems_found += 1
        map.show :secret_door if total > 8
        #map.show :secret_door if total / map.total_gems.to_f > 0.5
      end
      #on_collide ?
    end

    # on_action do |action|
    #   case action
    #   when :accept
    #     if colliding_object && colliding_object.dialogue
    #       say colliding_object.dialogue
    #     end
    #   end
    # end

  end

  # example of context of 'map' for setup
  map do
    define gems_found: 0, total_gems: 0
    on_load do
      say @filename # dialogue file reference for this map
      # @total_gems = objects_by_name('gem').length
      # Clear inventory for new map.
      players.each {|pl| pl.num_gems = 0 }
    end

    # on_enter { say "here again?!?" }

    # if filename =~ /water/
    #   player.animation = :swimming
    # end
  end
